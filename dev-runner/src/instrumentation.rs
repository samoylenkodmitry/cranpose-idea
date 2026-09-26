use anyhow::{Context, Result};
use syn::spanned::Spanned;

/// Adds development attributes without moving the source's existing line numbers.
pub fn instrument(source: &str) -> Result<String> {
    let file = syn::parse_file(source).context("parse Rust source for hot reload")?;
    let mut insertions = Vec::new();
    collect(&file.items, &mut insertions);
    let lines: Vec<_> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
        .collect();
    let mut output = source.to_owned();
    insertions.sort_unstable();
    for (line, column) in insertions.into_iter().rev() {
        let offset = lines[line - 1] + column;
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
