use crate::instrumentation::instrument_file;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::Digest;
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

impl Target {
    pub(crate) fn is_library(&self) -> bool {
        self.kind.iter().any(|kind| {
            matches!(
                kind.as_str(),
                "lib" | "rlib" | "dylib" | "cdylib" | "staticlib"
            )
        })
    }
}

impl Metadata {
    /// Reads workspace members and their declared targets.
    /// Whether the application's Cranpose offers the development-only
    /// `hot-reload` feature, which keys composition by source structure so hot
    /// patches keep state around an edit. The dependency's own manifest decides:
    /// a path dependency, or the git checkout or registry source Cargo.lock
    /// names under `CARGO_HOME`. Releases from 0.1.175 are known to have it.
    /// Otherwise (no lockfile, or the source is not fetched yet) Cargo resolves
    /// the private copy at `resolve_in` and reports the package's features.
    pub fn cranpose_hot_reload(&self, dependency: &Dependency, resolve_in: Option<&Path>) -> bool {
        if let Some(known) = self.locked_hot_reload(dependency, resolve_in) {
            return known;
        }
        resolve_in
            .and_then(|root| resolved_feature(root, &dependency.name, "hot-reload"))
            .unwrap_or(false)
    }

    fn locked_hot_reload(
        &self,
        dependency: &Dependency,
        resolve_in: Option<&Path>,
    ) -> Option<bool> {
        if let Some(path) = &dependency.path {
            return Some(manifest_offers(&path.join("Cargo.toml")).unwrap_or(false));
        }
        let lock = resolve_in
            .into_iter()
            .chain([self.workspace_root.as_path()])
            .find_map(|root| {
                fs::read_to_string(root.join("Cargo.lock"))
                    .ok()?
                    .parse::<toml_edit::DocumentMut>()
                    .ok()
            })?;
        let cargo_home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
        let mut answer = None;
        for package in lock
            .get("package")
            .and_then(toml_edit::Item::as_array_of_tables)
            .into_iter()
            .flatten()
            .filter(|package| {
                package.get("name").and_then(toml_edit::Item::as_str) == Some(&dependency.name)
            })
        {
            let text = |key| package.get(key).and_then(toml_edit::Item::as_str);
            let version = text("version").unwrap_or_default();
            let source = text("source").unwrap_or_default();
            let offers = cargo_home
                .as_deref()
                .and_then(|home| locked_manifest(home, &dependency.name, version, source))
                .and_then(|manifest| manifest_offers(&manifest));
            let released = {
                let mut parts = version
                    .split(['.', '-', '+'])
                    .map(|part| part.parse::<u64>().unwrap_or(0));
                [(); 3].map(|_| parts.next().unwrap_or(0)) >= [0, 1, 175]
            };
            match offers {
                Some(true) => return Some(true),
                _ if released => return Some(true),
                Some(false) => answer = Some(false),
                None => {}
            }
        }
        answer
    }

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

fn manifest_offers(manifest: &Path) -> Option<bool> {
    let manifest = fs::read_to_string(manifest)
        .ok()?
        .parse::<toml_edit::DocumentMut>()
        .ok()?;
    Some(
        manifest
            .get("features")
            .and_then(|features| features.get("hot-reload"))
            .is_some(),
    )
}

/// Whether Cargo's resolved graph for `root` gives package `name` `feature`.
fn resolved_feature(root: &Path, name: &str, feature: &str) -> Option<bool> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    Some(
        metadata["packages"]
            .as_array()?
            .iter()
            .filter(|package| package["name"] == name)
            .any(|package| package["features"].get(feature).is_some()),
    )
}

/// The manifest of a locked registry or git package in Cargo's source caches.
fn locked_manifest(home: &Path, name: &str, version: &str, source: &str) -> Option<PathBuf> {
    let entries = |dir: PathBuf| fs::read_dir(dir).into_iter().flatten().flatten();
    if source.starts_with("registry+") || source.starts_with("sparse+") {
        return entries(home.join("registry/src"))
            .map(|index| index.path().join(format!("{name}-{version}/Cargo.toml")))
            .find(|manifest| manifest.is_file());
    }
    let revision = source.strip_prefix("git+")?.rsplit_once('#')?.1;
    let short = revision.get(..7)?;
    entries(home.join("git/checkouts"))
        .map(|repository| repository.path().join(short))
        .filter(|checkout| checkout.is_dir())
        .flat_map(|checkout| walkdir::WalkDir::new(checkout).max_depth(3))
        .flatten()
        .map(walkdir::DirEntry::into_path)
        .filter(|path| path.file_name().is_some_and(|file| file == "Cargo.toml"))
        .find(|manifest| {
            fs::read_to_string(manifest)
                .ok()
                .and_then(|text| text.parse::<toml_edit::DocumentMut>().ok())
                .is_some_and(|manifest| {
                    manifest
                        .get("package")
                        .and_then(|package| package.get("name"))
                        .and_then(toml_edit::Item::as_str)
                        == Some(name)
                })
        })
}

/// A private copy used exclusively by the plugin's development compiler.
#[derive(Debug)]
pub struct DevWorkspace {
    pub original: PathBuf,
    pub directory: PathBuf,
    pub sources: BTreeMap<PathBuf, String>,
    /// SHA-256 of other copied files up to 4 MiB, so identical rewrites never rebuild.
    pub files: BTreeMap<PathBuf, Vec<u8>>,
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
        let mut files = BTreeMap::new();
        let mut manifests = Vec::new();
        for entry in walkdir::WalkDir::new(&original)
            .into_iter()
            .filter_entry(|entry| {
                !ignored_path(entry.path().strip_prefix(&original).unwrap_or(entry.path()))
                    && entry.path() != metadata.target_directory
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
                } else if entry.metadata()?.len() <= 4 << 20 {
                    let digest = sha2::Sha256::digest(fs::read(entry.path())?);
                    files.insert(relative.to_owned(), digest.to_vec());
                }
            }
        }
        for (source, destination) in manifests {
            let mut document = fs::read_to_string(&source)?.parse::<DocumentMut>()?;
            let base = source.parent().context("manifest directory")?;
            external_paths(document.as_table_mut(), base, &original);
            fs::write(destination, document.to_string())?;
        }
        let support = if hot_reload {
            Some(write_support(support)?)
        } else {
            None
        };
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
                if !target.is_library()
                    && !target
                        .kind
                        .iter()
                        .any(|kind| matches!(kind.as_str(), "bin" | "example"))
                {
                    continue;
                }
                if let Ok(relative) = target.src_path.canonicalize()?.strip_prefix(&original) {
                    roots.insert(relative.to_owned(), alias.clone());
                }
            }
            instrument_manifest(
                &directory.join(manifest),
                support.as_deref().context("hot-reload support")?,
            )?;
        }
        let workspace = Self {
            original,
            directory,
            sources,
            files,
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
            instrument_file(source, &relative.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| source.to_owned())
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

fn write_support(directory: &Path) -> Result<PathBuf> {
    let root = directory
        .parent()
        .context("support directory")?
        .join("bundles");
    let mut manifest = include_str!("../../dev-macros/Cargo.toml").parse::<DocumentMut>()?;
    manifest.remove("lints");
    manifest["workspace"] = Item::Table(toml_edit::Table::new());
    let manifest = manifest.to_string();
    let bundle = cranpose_plugin_cache::materialize(&root, &[
        ("dev-macros/Cargo.toml", manifest.as_bytes()),
        ("dev-macros/src/lib.rs", include_bytes!("../../dev-macros/src/lib.rs")),
        ("runtime/Cargo.toml", b"[package]\nname='cranpose-dev-runtime'\nversion='0.1.0'\nedition='2024'\n[workspace]\n[dependencies]\nsubsecond='=0.7.10'\ndioxus-devtools='=0.7.10'\nserde={version='1',features=['derive']}\nserde_json='1'\nrand='0.9'\n"),
        ("runtime/src/lib.rs", include_bytes!("../assets/shared_runtime.rs")),
        ("runtime/src/values.rs", cranpose_plugin_authoring::RUNTIME_SOURCE.as_bytes()),
        ("runtime/src/transport.rs", cranpose_plugin_authoring::TRANSPORT_SOURCE.as_bytes()),
    ])?;
    Ok(bundle.join("dev-macros"))
}

#[cfg(test)]
#[path = "../tests/unit/workspace.rs"]
mod tests;

/// Directories excluded from both the private copy and change observation.
pub(crate) fn ignored_path(path: &Path) -> bool {
    path.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some(
                ".git"
                    | ".idea"
                    | "target"
                    | "node_modules"
                    | ".gradle"
                    | ".intellijPlatform"
                    | ".venv"
            )
        )
    })
}
