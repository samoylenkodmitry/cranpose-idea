use anyhow::{Context, Result};
use syn::spanned::Spanned;

/// Adds development attributes without moving the source's existing line numbers.
pub fn instrument(source: &str) -> Result<String> {
    instrument_file(source, "")
}

pub fn instrument_file(source: &str, file_key: &str) -> Result<String> {
    let catalog = cranpose_plugin_authoring::Catalog::parse(source)?;
    let source = catalog.instrument(source, file_key);
    let source = source.as_str();
    let file = syn::parse_file(source).context("parse Rust source for hot reload")?;
    let mut insertions = Vec::new();
    collect(&file.items, &mut insertions);
    let mut output = source.to_owned();
    insertions.sort_unstable();
    for (line, column) in insertions.into_iter().rev() {
        let offset = byte_offset(source, line, column);
        output.insert_str(offset, "#[cranpose_dev_macros::hot] ");
    }
    Ok(output)
}

fn collect(items: &[syn::Item], insertions: &mut Vec<(usize, usize)>) {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                if let Some(attribute) = function.attrs.iter().find(|attribute| {
                    attribute
                        .path()
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == "composable")
                }) {
                    let start = attribute.span().start();
                    insertions.push((start.line, start.column));
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect(items, insertions);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/instrumentation.rs"]
mod tests;

pub(crate) fn byte_offset(source: &str, line: usize, column: usize) -> usize {
    let base = source
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    base + source
        .get(base..)
        .unwrap_or_default()
        .chars()
        .take(column)
        .map(char::len_utf8)
        .sum::<usize>()
}
