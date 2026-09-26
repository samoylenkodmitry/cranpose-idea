use quote::ToTokens;
use serde::Serialize;
use syn::visit_mut::{self, VisitMut};

/// Whether a saved edit can reuse the running application's typed state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "reason", rename_all = "camelCase")]
pub enum ReloadDecision {
    Unchanged,
    Patch,
    Restart(String),
    Invalid(String),
}

/// Accepts literal edits with unchanged types, captures, call sites and declarations.
/// Other edits require a fresh process before any new code is executed.
pub fn classify(previous: &str, next: &str) -> ReloadDecision {
    if previous == next {
        return ReloadDecision::Unchanged;
    }
    let old = match signature(previous) {
        Ok(value) => value,
        Err(error) => return ReloadDecision::Restart(error.to_string()),
    };
    let new = match signature(next) {
        Ok(value) => value,
        Err(error) => return ReloadDecision::Invalid(error.to_string()),
    };
    if old == new {
        ReloadDecision::Patch
    } else {
        ReloadDecision::Restart("Code structure, captures, types or state locations changed".into())
    }
}

fn signature(source: &str) -> syn::Result<(String, Vec<(usize, usize)>)> {
    let mut file = syn::parse_file(source)?;
    let mut normalize = Normalize::default();
    normalize.visit_file_mut(&mut file);
    Ok((file.into_token_stream().to_string(), normalize.calls))
}

#[derive(Default)]
struct Normalize {
    calls: Vec<(usize, usize)>,
}

impl VisitMut for Normalize {
    fn visit_item_fn_mut(&mut self, function: &mut syn::ItemFn) {
        if function.sig.ident != "main" && function.sig.constness.is_none() {
            self.visit_block_mut(&mut function.block);
        }
    }
    fn visit_item_const_mut(&mut self, _: &mut syn::ItemConst) {}
    fn visit_item_static_mut(&mut self, _: &mut syn::ItemStatic) {}
    fn visit_type_mut(&mut self, _: &mut syn::Type) {}
    fn visit_generic_argument_mut(&mut self, _: &mut syn::GenericArgument) {}
    fn visit_expr_const_mut(&mut self, _: &mut syn::ExprConst) {}
    fn visit_expr_repeat_mut(&mut self, repeat: &mut syn::ExprRepeat) {
        self.visit_expr_mut(&mut repeat.expr);
    }
    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        use syn::spanned::Spanned;
        let location = call.span().start();
        self.calls.push((location.line, location.column));
        visit_mut::visit_expr_call_mut(self, call);
    }
    fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
        let location = call.method.span().start();
        self.calls.push((location.line, location.column));
        visit_mut::visit_expr_method_call_mut(self, call);
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
    }
}

#[cfg(test)]
#[path = "../tests/unit/policy.rs"]
mod tests;
