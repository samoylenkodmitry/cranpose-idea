use crate::instrumentation::instrument;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use toml_edit::{DocumentMut, InlineTable, Item, Value, value};

/// Cargo's workspace information, read without running a build in the source tree.
#[derive(Clone, Debug, Deserialize)]
pub struct Metadata {
    pub workspace_root: PathBuf,
    pub target_directory: PathBuf,
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub rename: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub name: String,
    pub kind: Vec<String>,
    pub src_path: PathBuf,
    #[serde(rename = "required-features", default)]
    pub required_features: Vec<String>,
}

impl Metadata {
    /// Reads workspace members and their declared targets.
    pub fn read(root: &Path) -> Result<Self> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version=1", "--no-deps"])
            .current_dir(root)
            .output()
            .context("start Cargo metadata")?;
        if !output.status.success() {
            bail!("{}", String::from_utf8_lossy(&output.stderr));
        }
        serde_json::from_slice(&output.stdout).context("decode Cargo metadata")
    }
}

/// A private copy used exclusively by the plugin's development compiler.
#[derive(Debug)]
pub struct DevWorkspace {
    pub original: PathBuf,
    pub directory: PathBuf,
    pub sources: BTreeMap<PathBuf, String>,
    roots: BTreeMap<PathBuf, String>,
    packages: BTreeMap<PathBuf, bool>,
    copies: Vec<(PathBuf, PathBuf)>,
}

impl DevWorkspace {
    /// Copies a workspace and instruments its Cranpose packages. Never writes to the source tree.
    pub fn prepare(metadata: &Metadata, directory: &Path, support: &Path) -> Result<Self> {
        Self::prepare_with_mode(metadata, directory, support, true)
    }

    /// Ordinary previews share the private copy, with no hot-reload dependencies or instrumentation.
    pub fn prepare_with_mode(
        metadata: &Metadata,
        directory: &Path,
        support: &Path,
        hot_reload: bool,
    ) -> Result<Self> {
        let original = metadata.workspace_root.canonicalize()?;
        if directory.starts_with(&original) {
            bail!("development cache must be outside the application workspace");
        }
        fs::create_dir_all(directory)?;
        let directory = directory.canonicalize()?;
        if directory.starts_with(&original) {
            bail!("development cache resolves inside the application workspace");
        }
        let mut sources = BTreeMap::new();
        let mut manifests = Vec::new();
        for entry in walkdir::WalkDir::new(&original)
            .into_iter()
            .filter_entry(|entry| {
                !matches!(
                    entry.file_name().to_str(),
                    Some(
                        ".git"
                            | ".idea"
                            | "target"
                            | "node_modules"
                            | ".gradle"
                            | ".intellijPlatform"
                            | ".venv"
                    )
                ) && entry.path() != metadata.target_directory
            })
        {
            let entry = entry?;
            let relative = entry.path().strip_prefix(&original)?;
            if entry.file_type().is_symlink() {
                bail!("symlink {} requires a normal build", relative.display());
            }
            let destination = directory.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&destination)?;
            } else if entry.file_type().is_file() {
                fs::copy(entry.path(), &destination)?;
                if entry.file_name() == "Cargo.toml" {
                    manifests.push((entry.path().to_owned(), destination.clone()));
                }
                if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "rs")
                {
                    sources.insert(relative.to_owned(), fs::read_to_string(entry.path())?);
                }
            }
        }
        for (source, destination) in manifests {
            let mut document = fs::read_to_string(&source)?.parse::<DocumentMut>()?;
            let base = source.parent().context("manifest directory")?;
            external_paths(document.as_table_mut(), base, &original);
            fs::write(destination, document.to_string())?;
        }
        if hot_reload {
            write_support(support)?;
        }
        let mut roots = BTreeMap::new();
        let mut packages = BTreeMap::new();
        for package in &metadata.packages {
            let package_manifest = package.manifest_path.canonicalize()?;
            let relative_manifest = package_manifest
                .strip_prefix(&original)
                .context("workspace package is outside its root")?;
            let package_root = relative_manifest
                .parent()
                .context("package manifest has no parent")?
                .to_owned();
            packages.insert(
                package_root,
                hot_reload
                    && package
                        .dependencies
                        .iter()
                        .any(|dependency| dependency.name == "cranpose"),
            );
            if !hot_reload {
                continue;
            }
            let Some(dependency) = package
                .dependencies
                .iter()
                .find(|dependency| dependency.name == "cranpose")
            else {
                continue;
            };
            let manifest = relative_manifest;
            let alias = dependency
                .rename
                .as_deref()
                .unwrap_or("cranpose")
                .replace('-', "_");
            for target in &package.targets {
                if !target
                    .kind
                    .iter()
                    .any(|kind| matches!(kind.as_str(), "bin" | "lib" | "example"))
                {
                    continue;
                }
                if let Ok(relative) = target.src_path.canonicalize()?.strip_prefix(&original) {
                    roots.insert(relative.to_owned(), alias.clone());
                }
            }
            instrument_manifest(&directory.join(manifest), support)?;
        }
        let workspace = Self {
            original,
            directory,
            sources,
            roots,
            packages,
            copies: Vec::new(),
        };
        for (relative, source) in &workspace.sources {
            workspace.write_source(relative, source)?;
        }
        Ok(workspace)
    }

    /// Writes a validated edit into the private tree while preserving source line numbers.
    pub fn write_source(&self, relative: &Path, source: &str) -> Result<()> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            bail!("source path escaped the development workspace");
        }
        let enabled = self
            .packages
            .iter()
            .filter(|(root, _)| relative.starts_with(root))
            .max_by_key(|(root, _)| root.components().count())
            .is_some_and(|(_, enabled)| *enabled);
        let mut output = if enabled {
            instrument(source).unwrap_or_else(|_| source.to_owned())
        } else {
            source.to_owned()
        };
        if let Some(alias) = self.roots.get(relative) {
            let file = syn::parse_file(&output)?;
            use syn::spanned::Spanned;
            let start = file.items.first().map(|item| item.span().start());
            let offset = start
                .map(|start| crate::instrumentation::byte_offset(&output, start.line, start.column))
                .unwrap_or(output.len());
            output.insert_str(offset, &format!("extern crate {alias} as __cranpose_api; #[allow(dead_code)] mod __cranpose_dev {{ include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/.cranpose-dev/runtime.rs\")); }} "));
        }
        fs::write(self.directory.join(relative), &output)?;
        for (source, destination) in &self.copies {
            if let Ok(relative) = relative.strip_prefix(source) {
                fs::write(destination.join(relative), &output)?;
            }
        }
        Ok(())
    }

    pub(crate) fn add_copy(&mut self, source: PathBuf, destination: PathBuf) {
        self.copies.push((source, destination));
    }

    pub(crate) fn source_maps(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut maps: Vec<_> = self
            .copies
            .iter()
            .map(|(source, destination)| (destination.clone(), self.original.join(source)))
            .collect();
        for (source, destination) in &self.copies {
            if let Ok(relative) = destination.strip_prefix(&self.directory) {
                maps.push((relative.to_owned(), self.original.join(source)));
            }
        }
        maps.push((self.directory.clone(), self.original.clone()));
        maps
    }
}

fn instrument_manifest(path: &Path, support: &Path) -> Result<()> {
    let mut document = fs::read_to_string(path)?.parse::<DocumentMut>()?;
    let mut macro_dependency = InlineTable::new();
    macro_dependency.insert("path", Value::from(support.to_string_lossy().as_ref()));
    document["dependencies"]["cranpose-dev-macros"] =
        Item::Value(Value::InlineTable(macro_dependency));
    let mut runtime_dependency = InlineTable::new();
    runtime_dependency.insert(
        "path",
        Value::from(
            support
                .parent()
                .context("support directory")?
                .join("runtime")
                .to_string_lossy()
                .as_ref(),
        ),
    );
    document["dependencies"]["cranpose-dev-runtime"] =
        Item::Value(Value::InlineTable(runtime_dependency));
    fs::write(path, document.to_string())?;
    let runtime = path
        .parent()
        .context("manifest has no directory")?
        .join(".cranpose-dev");
    fs::create_dir_all(&runtime)?;
    fs::write(
        runtime.join("runtime.rs"),
        include_str!("../assets/runtime.rs"),
    )?;
    Ok(())
}

// Relative dependencies outside the copied workspace must keep resolving to the real path.
fn external_paths(table: &mut toml_edit::Table, base: &Path, original: &Path) {
    for (key, item) in table.iter_mut() {
        if key == "path" {
            if let Some(path) = item
                .as_str()
                .and_then(|path| base.join(path).canonicalize().ok())
                .filter(|path| !path.starts_with(original))
            {
                *item = value(path.to_string_lossy().as_ref());
            }
        } else if let Some(child) = item.as_table_mut() {
            external_paths(child, base, original);
        } else if let Some(child) = item.as_inline_table_mut() {
            external_inline_paths(child, base, original);
        } else if let Some(children) = item.as_array_of_tables_mut() {
            for child in children.iter_mut() {
                external_paths(child, base, original);
            }
        }
    }
}
fn external_inline_paths(table: &mut InlineTable, base: &Path, original: &Path) {
    for (key, item) in table.iter_mut() {
        if key == "path" {
            if let Some(path) = item
                .as_str()
                .and_then(|path| base.join(path).canonicalize().ok())
                .filter(|path| !path.starts_with(original))
            {
                *item = Value::from(path.to_string_lossy().as_ref());
            }
        } else if let Some(child) = item.as_inline_table_mut() {
            external_inline_paths(child, base, original);
        }
    }
}

fn write_support(directory: &Path) -> Result<()> {
    let runtime = directory
        .parent()
        .context("support directory")?
        .join("runtime");
    fs::create_dir_all(runtime.join("src"))?;
    fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='cranpose-dev-runtime'\nversion='0.1.0'\nedition='2024'\n[workspace]\n[dependencies]\nsubsecond='=0.7.10'\ndioxus-devtools='=0.7.10'\n",
    )?;
    fs::write(
        runtime.join("src/lib.rs"),
        include_str!("../assets/shared_runtime.rs"),
    )?;
    fs::create_dir_all(directory.join("src"))?;
    let mut manifest = include_str!("../../dev-macros/Cargo.toml").parse::<DocumentMut>()?;
    manifest.remove("lints");
    manifest["workspace"] = Item::Table(toml_edit::Table::new());
    fs::write(directory.join("Cargo.toml"), manifest.to_string())?;
    fs::write(
        directory.join("src/lib.rs"),
        include_str!("../../dev-macros/src/lib.rs"),
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/workspace.rs"]
mod tests;
