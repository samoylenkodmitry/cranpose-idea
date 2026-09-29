use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::Serialize;
use std::collections::BTreeMap;
use syn::{
    visit::Visit,
    visit_mut::{self, VisitMut},
};

/// Whether a saved edit can reuse the running application's typed state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "reason", rename_all = "camelCase")]
pub enum ReloadDecision {
    Unchanged,
    Patch,
    Restart(String),
    Invalid(String),
}

/// Compares `next` with the source compiled into the running process. Function bodies,
/// new functions and imports are hot-patched. Everything old code shares with new code
/// (layouts, signatures, statics, consts, traits, impl headers, macros, modules and
/// `main`) must stay identical; literal type changes still rebuild to keep state.
pub fn classify(previous: &str, next: &str) -> ReloadDecision {
    if previous == next {
        return ReloadDecision::Unchanged;
    }
    let next = match syn::parse_file(next) {
        Ok(file) => file,
        Err(error) => {
            let line = error.span().start().line;
            return ReloadDecision::Invalid(if line > 0 {
                format!("line {line}: {error}")
            } else {
                error.to_string()
            });
        }
    };
    let Ok(previous) = syn::parse_file(previous) else {
        return ReloadDecision::Restart("Previous source had a syntax error".into());
    };
    match Interface::of(&previous).compare(&Interface::of(&next)) {
        Some(reason) => ReloadDecision::Restart(reason),
        None => ReloadDecision::Patch,
    }
}

#[derive(Default)]
struct Interface {
    entries: BTreeMap<String, Entry>,
    order: Vec<String>,
}

struct Entry {
    label: String,
    changed: String,
    attributes: String,
    signature: String,
    body: Option<Body>,
    detachable: bool,
    plain: bool,
}

struct Body {
    name: String,
    shape: String,
    literals: String,
}

impl Interface {
    fn of(file: &syn::File) -> Self {
        let mut interface = Self::default();
        for item in &file.items {
            interface.item(item, "", true);
        }
        interface
    }

    fn compare(&self, next: &Self) -> Option<String> {
        let removed = self
            .order
            .iter()
            .filter(|key| !next.entries.contains_key(*key));
        for key in next.order.iter().chain(removed) {
            match (self.entries.get(key), next.entries.get(key)) {
                (Some(old), Some(new)) => {
                    if old.attributes != new.attributes {
                        return Some(format!("Attributes of {} changed", new.label));
                    }
                    if old.signature != new.signature {
                        return Some(new.changed.clone());
                    }
                    if let (Some(old), Some(new)) = (&old.body, &new.body)
                        && old.shape == new.shape
                        && old.literals != new.literals
                    {
                        return Some(format!("Literal types changed in `{}`", new.name));
                    }
                }
                (None, Some(new)) if !(new.detachable && new.plain) => {
                    return Some(format!("{} added", capitalized(&new.label)));
                }
                (Some(old), None) if !old.detachable => {
                    return Some(format!("{} removed", capitalized(&old.label)));
                }
                _ => {}
            }
        }
        None
    }

    fn insert(&mut self, key: String, entry: Entry) -> String {
        let mut unique = key.clone();
        let mut occurrence = 1;
        while self.entries.contains_key(&unique) {
            occurrence += 1;
            unique = format!("{key}#{occurrence}");
        }
        self.order.push(unique.clone());
        self.entries.insert(unique.clone(), entry);
        unique
    }

    fn fixed(
        &mut self,
        key: String,
        label: String,
        changed: String,
        attrs: &[syn::Attribute],
        tokens: String,
    ) -> String {
        self.insert(
            key,
            Entry {
                label,
                changed,
                attributes: attributes(attrs),
                signature: tokens,
                body: None,
                detachable: false,
                plain: false,
            },
        )
    }

    fn function(
        &mut self,
        key: String,
        name: String,
        attrs: &[syn::Attribute],
        signature: &syn::Signature,
        block: &syn::Block,
        detachable: bool,
    ) {
        let preview = attrs
            .iter()
            .any(|attribute| last(attribute.path()) == "preview");
        let key = self.insert(
            key,
            Entry {
                label: format!("{}`{name}`", if preview { "preview " } else { "function " }),
                changed: format!("Signature of `{name}` changed"),
                attributes: attributes(attrs),
                signature: text(signature),
                body: Some(Body::of(block, &name)),
                detachable: detachable && !preview,
                plain: attrs.iter().all(|attribute| {
                    matches!(
                        last(attribute.path()).as_str(),
                        "composable"
                            | "doc"
                            | "allow"
                            | "expect"
                            | "warn"
                            | "deny"
                            | "forbid"
                            | "inline"
                            | "must_use"
                            | "track_caller"
                            | "cold"
                            | "cfg"
                            | "deprecated"
                    )
                }),
            },
        );
        self.nested(block, &key);
    }

    fn nested(&mut self, block: &syn::Block, scope: &str) {
        let mut nested = Nested::default();
        nested.visit_block(block);
        let scope = format!("{scope}::");
        for item in nested.items {
            self.item(item, &scope, false);
        }
        for statement in nested.macros {
            self.fixed(
                format!("{scope}thread_local!"),
                "`thread_local!` block".into(),
                "`thread_local!` block changed".into(),
                &statement.attrs,
                text(&statement.mac),
            );
        }
    }

    /// Types, statics, constants and traits are shared with old code as declared.
    fn declared(
        &mut self,
        scope: &str,
        (kind, noun, change): (&str, &str, &str),
        name: &syn::Ident,
        attrs: &[syn::Attribute],
        item: &syn::Item,
    ) {
        let mut bare = item.clone();
        clear_attributes(&mut bare);
        self.fixed(
            format!("{scope}{kind} {name}"),
            format!("{noun} `{name}`"),
            format!("{} `{name}` {change}", capitalized(noun)),
            attrs,
            text(&bare),
        );
    }

    fn item(&mut self, item: &syn::Item, scope: &str, root: bool) {
        if excluded(item_attributes(item)) {
            return;
        }
        match item {
            syn::Item::Use(_) => {}
            syn::Item::Fn(function) if root && function.sig.ident == "main" => {
                let mut bare = function.clone();
                bare.attrs.clear();
                let key = format!("{scope}fn main");
                self.fixed(
                    key,
                    "`main`".into(),
                    "`main` changed".into(),
                    &function.attrs,
                    text(&bare),
                );
            }
            syn::Item::Fn(function) if function.sig.constness.is_some() => {
                let name = &function.sig.ident;
                let mut bare = function.clone();
                bare.attrs.clear();
                let key = self.fixed(
                    format!("{scope}fn {name}"),
                    format!("const fn `{name}`"),
                    format!("const fn `{name}` changed"),
                    &function.attrs,
                    text(&bare),
                );
                self.nested(&function.block, &key);
            }
            syn::Item::Fn(function) => {
                let name = function.sig.ident.to_string();
                let key = format!("{scope}fn {name}");
                self.function(
                    key,
                    name,
                    &function.attrs,
                    &function.sig,
                    &function.block,
                    true,
                );
            }
            syn::Item::Impl(implementation) => {
                let self_ty = compact(&text(&implementation.self_ty));
                let title = match &implementation.trait_ {
                    Some((path, _)) => format!("{} for {self_ty}", compact(&text(path))),
                    None => self_ty.clone(),
                };
                let mut header = implementation.clone();
                header.attrs.clear();
                header.items.clear();
                let key = self.fixed(
                    format!("{scope}impl {title}"),
                    format!("impl `{title}`"),
                    format!("Header of impl `{title}` changed"),
                    &implementation.attrs,
                    text(&header),
                );
                for member in &implementation.items {
                    self.member(member, &key, &self_ty, implementation.trait_.is_none());
                }
            }
            syn::Item::Mod(module) => {
                let name = &module.ident;
                let mut declaration = module.clone();
                declaration.attrs.clear();
                if let Some((_, items)) = &mut declaration.content {
                    items.clear();
                }
                self.fixed(
                    format!("{scope}mod {name}"),
                    format!("module `{name}`"),
                    format!("Module `{name}` declaration changed"),
                    &module.attrs,
                    text(&declaration),
                );
                if let Some((_, items)) = &module.content {
                    let scope = format!("{scope}{name}::");
                    for item in items {
                        self.item(item, &scope, false);
                    }
                }
            }
            syn::Item::Struct(value) => {
                let kind = ("struct", "struct", "fields changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Enum(value) => {
                let kind = ("enum", "enum", "variants changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Union(value) => {
                let kind = ("union", "union", "fields changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Type(value) => {
                let kind = ("type", "type alias", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Static(value) => {
                let kind = ("static", "static", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Const(value) => {
                let kind = ("const", "constant", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Trait(value) => {
                let kind = ("trait", "trait", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::TraitAlias(value) => {
                let kind = ("trait", "trait", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::ExternCrate(value) => {
                let kind = ("extern crate", "extern crate", "changed");
                self.declared(scope, kind, &value.ident, &value.attrs, item);
            }
            syn::Item::Macro(value) => match &value.ident {
                Some(name) => {
                    let kind = ("macro", "macro", "changed");
                    self.declared(scope, kind, name, &value.attrs, item);
                }
                None => {
                    let path = compact(&text(&value.mac.path));
                    let mut bare = value.clone();
                    bare.attrs.clear();
                    self.fixed(
                        format!("{scope}{path}!"),
                        format!("`{path}!` block"),
                        format!("`{path}!` block changed"),
                        &value.attrs,
                        text(&bare),
                    );
                }
            },
            _ => {
                let key = format!("{scope}item");
                self.fixed(
                    key,
                    "item".into(),
                    "Declarations changed".into(),
                    &[],
                    text(item),
                );
            }
        }
    }

    fn member(&mut self, member: &syn::ImplItem, scope: &str, self_ty: &str, inherent: bool) {
        let (key, noun, attrs) = match member {
            syn::ImplItem::Fn(function) => {
                if !excluded(&function.attrs) {
                    self.function(
                        format!("{scope}::fn {}", function.sig.ident),
                        format!("{self_ty}::{}", function.sig.ident),
                        &function.attrs,
                        &function.sig,
                        &function.block,
                        inherent,
                    );
                }
                return;
            }
            syn::ImplItem::Const(value) => (
                format!("const {}", value.ident),
                "constant",
                &value.attrs[..],
            ),
            syn::ImplItem::Type(value) => (
                format!("type {}", value.ident),
                "associated type",
                &value.attrs[..],
            ),
            _ => ("item".into(), "item", &[][..]),
        };
        let mut bare = member.clone();
        match &mut bare {
            syn::ImplItem::Const(value) => value.attrs.clear(),
            syn::ImplItem::Type(value) => value.attrs.clear(),
            _ => {}
        }
        let name = format!("{self_ty}::{}", key.rsplit(' ').next().unwrap_or_default());
        self.fixed(
            format!("{scope}::{key}"),
            format!("{noun} `{name}`"),
            format!("{} `{name}` changed", capitalized(noun)),
            attrs,
            text(&bare),
        );
    }
}

impl Body {
    fn of(block: &syn::Block, name: &str) -> Self {
        let mut shape = block.clone();
        Erase.visit_block_mut(&mut shape);
        let mut literals = block.clone();
        LiteralTypes.visit_block_mut(&mut literals);
        Self {
            name: name.into(),
            shape: text(&shape),
            literals: text(&literals),
        }
    }
}

/// Items and `thread_local!` declarations inside function bodies share the interface.
#[derive(Default)]
struct Nested<'a> {
    items: Vec<&'a syn::Item>,
    macros: Vec<&'a syn::StmtMacro>,
}
impl<'a> Visit<'a> for Nested<'a> {
    fn visit_item(&mut self, item: &'a syn::Item) {
        self.items.push(item);
    }
    fn visit_stmt_macro(&mut self, statement: &'a syn::StmtMacro) {
        if last(&statement.mac.path) == "thread_local" {
            self.macros.push(statement);
        }
    }
}

/// Every literal becomes one placeholder: equal shapes differ only in literals.
struct Erase;
impl VisitMut for Erase {
    fn visit_lit_mut(&mut self, literal: &mut syn::Lit) {
        *literal = syn::Lit::Int(syn::LitInt::new("0", literal.span()));
    }
}

/// Keeps literal kinds, suffixes and literals that determine types (array lengths,
/// const arguments). A change here alters the type of a value, such as remembered state.
struct LiteralTypes;
impl VisitMut for LiteralTypes {
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}
    fn visit_type_mut(&mut self, _: &mut syn::Type) {}
    fn visit_generic_argument_mut(&mut self, _: &mut syn::GenericArgument) {}
    fn visit_expr_const_mut(&mut self, _: &mut syn::ExprConst) {}
    fn visit_expr_repeat_mut(&mut self, repeat: &mut syn::ExprRepeat) {
        self.visit_expr_mut(&mut repeat.expr);
    }
    fn visit_expr_lit_mut(&mut self, expression: &mut syn::ExprLit) {
        match &mut expression.lit {
            syn::Lit::Str(value) => *value = syn::LitStr::new("", value.span()),
            syn::Lit::Int(value) => {
                *value = syn::LitInt::new(&format!("0{}", value.suffix()), value.span())
            }
            syn::Lit::Float(value) => {
                *value = syn::LitFloat::new(&format!("0.0{}", value.suffix()), value.span())
            }
            syn::Lit::Bool(value) => value.value = false,
            syn::Lit::Char(value) => *value = syn::LitChar::new('a', value.span()),
            _ => {}
        }
        visit_mut::visit_expr_lit_mut(self, expression);
    }
}

fn excluded(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && attribute
                    .meta
                    .require_list()
                    .is_ok_and(|list| list.tokens.to_string() == "test"))
    })
}

fn item_attributes(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(value) => &value.attrs,
        syn::Item::Enum(value) => &value.attrs,
        syn::Item::ExternCrate(value) => &value.attrs,
        syn::Item::Fn(value) => &value.attrs,
        syn::Item::ForeignMod(value) => &value.attrs,
        syn::Item::Impl(value) => &value.attrs,
        syn::Item::Macro(value) => &value.attrs,
        syn::Item::Mod(value) => &value.attrs,
        syn::Item::Static(value) => &value.attrs,
        syn::Item::Struct(value) => &value.attrs,
        syn::Item::Trait(value) => &value.attrs,
        syn::Item::TraitAlias(value) => &value.attrs,
        syn::Item::Type(value) => &value.attrs,
        syn::Item::Union(value) => &value.attrs,
        syn::Item::Use(value) => &value.attrs,
        _ => &[],
    }
}

fn clear_attributes(item: &mut syn::Item) {
    match item {
        syn::Item::Const(value) => value.attrs.clear(),
        syn::Item::Enum(value) => value.attrs.clear(),
        syn::Item::ExternCrate(value) => value.attrs.clear(),
        syn::Item::Fn(value) => value.attrs.clear(),
        syn::Item::ForeignMod(value) => value.attrs.clear(),
        syn::Item::Impl(value) => value.attrs.clear(),
        syn::Item::Macro(value) => value.attrs.clear(),
        syn::Item::Mod(value) => value.attrs.clear(),
        syn::Item::Static(value) => value.attrs.clear(),
        syn::Item::Struct(value) => value.attrs.clear(),
        syn::Item::Trait(value) => value.attrs.clear(),
        syn::Item::TraitAlias(value) => value.attrs.clear(),
        syn::Item::Type(value) => value.attrs.clear(),
        syn::Item::Union(value) => value.attrs.clear(),
        syn::Item::Use(value) => value.attrs.clear(),
        _ => {}
    }
}

fn attributes(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .map(text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default()
}

/// Tokens without documentation, lint attributes or trailing commas in braces and
/// brackets, none of which change compiled code. `(T,)` keeps its comma.
fn text(node: &impl ToTokens) -> String {
    fn strip(tokens: TokenStream) -> TokenStream {
        let mut output: Vec<TokenTree> = Vec::new();
        let mut tokens = tokens.into_iter().peekable();
        while let Some(token) = tokens.next() {
            if let TokenTree::Punct(punct) = &token
                && punct.as_char() == '#'
                && let Some(TokenTree::Group(group)) = tokens.peek()
                && group.delimiter() == proc_macro2::Delimiter::Bracket
                && group.stream().into_iter().next().is_some_and(|first| {
                    matches!(
                        first.to_string().as_str(),
                        "doc" | "allow" | "expect" | "warn" | "deny" | "forbid"
                    )
                })
            {
                tokens.next();
                continue;
            }
            output.push(match token {
                TokenTree::Group(group) => {
                    let mut inner: Vec<_> = strip(group.stream()).into_iter().collect();
                    if matches!(
                        group.delimiter(),
                        proc_macro2::Delimiter::Brace | proc_macro2::Delimiter::Bracket
                    ) && matches!(inner.last(), Some(TokenTree::Punct(punct)) if punct.as_char() == ',')
                    {
                        inner.pop();
                    }
                    let mut stripped =
                        proc_macro2::Group::new(group.delimiter(), inner.into_iter().collect());
                    stripped.set_span(group.span());
                    TokenTree::Group(stripped)
                }
                token => token,
            });
        }
        output.into_iter().collect()
    }
    strip(node.to_token_stream()).to_string()
}

fn compact(text: &str) -> String {
    text.replace(" :: ", "::")
        .replace(":: ", "::")
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace("& ", "&")
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../tests/unit/policy.rs"]
mod tests;
