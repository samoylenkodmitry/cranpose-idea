use crate::workspace::{DevWorkspace, Package, Target};
use anyhow::{Context, Result};
use std::{fs, path::Path};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

/// Gives lib+bin packages a binary-only tip while preserving their real library as a workspace dependency.
pub fn prepare(
    workspace: &mut DevWorkspace,
    package: &Package,
    target: &Target,
) -> Result<Option<String>> {
    let Some(library) = package.targets.iter().find(|target| target.is_library()) else {
        return Ok(None);
    };
    let original_package = package
        .manifest_path
        .canonicalize()?
        .parent()
        .context("package directory")?
        .to_owned();
    let relative_package = original_package.strip_prefix(&workspace.original)?;
    let private_package = workspace.directory.join(relative_package);
    let launcher = workspace.directory.join(".cranpose-dev/launcher");
    fs::create_dir_all(&launcher)?;
    for entry in walkdir::WalkDir::new(&private_package)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".cranpose-dev")
    {
        let entry = entry?;
        let relative = entry.path().strip_prefix(&private_package)?;
        let destination = launcher.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), &destination)?;
        }
    }
    fs::create_dir_all(launcher.join(".cranpose-dev"))?;
    fs::copy(
        private_package.join(".cranpose-dev/runtime.rs"),
        launcher.join(".cranpose-dev/runtime.rs"),
    )?;
    let mut manifest = fs::read_to_string(launcher.join("Cargo.toml"))?.parse::<DocumentMut>()?;
    absolutize_paths(manifest.as_table_mut(), &private_package);
    for table in [
        "workspace",
        "lib",
        "bin",
        "example",
        "test",
        "bench",
        "profile",
        "patch",
        "replace",
    ] {
        manifest.remove(table);
    }
    let name = format!("cranpose-dev-launcher-{}", package.name);
    manifest["package"]["name"] = value(name.clone());
    manifest["package"]["workspace"] = value("../..");
    if let Some(package) = manifest["package"].as_table_mut() {
        package.remove("links");
        package.remove("default-run");
    }
    if target.kind.iter().any(|kind| kind == "example") {
        promote_example_dependencies(manifest.as_table_mut());
    }
    for key in [
        "autolib",
        "autobins",
        "autoexamples",
        "autotests",
        "autobenches",
    ] {
        manifest["package"][key] = value(false);
    }
    let mut binaries = ArrayOfTables::new();
    let mut binary = Table::new();
    binary["name"] = value(target.name.clone());
    binary["path"] = value(
        target
            .src_path
            .canonicalize()?
            .strip_prefix(&original_package)?
            .to_string_lossy()
            .as_ref(),
    );
    binaries.push(binary);
    manifest["bin"] = Item::ArrayOfTables(binaries);
    let alias = library.name.replace('-', "_");
    let mut library_dependency = InlineTable::new();
    library_dependency.insert("package", Value::from(package.name.clone()));
    library_dependency.insert(
        "path",
        Value::from(private_package.to_string_lossy().as_ref()),
    );
    manifest["dependencies"][&alias] = Item::Value(Value::InlineTable(library_dependency));
    for (feature, entries) in &package.features {
        let mut forwarded: Array = entries
            .iter()
            .map(|entry| Value::from(entry.as_str()))
            .collect();
        forwarded.push(format!("{alias}/{feature}"));
        manifest["features"][feature] = value(forwarded);
    }
    fs::write(launcher.join("Cargo.toml"), manifest.to_string())?;
    let root_manifest = workspace.directory.join("Cargo.toml");
    let mut root = fs::read_to_string(&root_manifest)?.parse::<DocumentMut>()?;
    if root
        .get("workspace")
        .and_then(|table| table.get("members"))
        .is_none()
    {
        root["workspace"]["members"] = value(Array::new());
    }
    root["workspace"]["members"]
        .as_array_mut()
        .context("workspace members must be an array")?
        .push(".cranpose-dev/launcher");
    fs::write(root_manifest, root.to_string())?;
    workspace.add_copy(relative_package.to_owned(), launcher.clone());
    println!(
        "{}",
        serde_json::json!({"cranposeDev": "sourceMap", "private": launcher, "original": original_package})
    );
    Ok(Some(name))
}

fn absolutize_paths(table: &mut Table, base: &Path) {
    for (key, item) in table.iter_mut() {
        if key == "path" {
            if let Some(path) = item.as_str().filter(|path| Path::new(path).is_relative()) {
                *item = value(base.join(path).to_string_lossy().as_ref());
            }
        } else if let Some(child) = item.as_table_mut() {
            absolutize_paths(child, base);
        } else if let Some(child) = item.as_inline_table_mut() {
            inline_paths(child, base);
        }
    }
}

fn promote_example_dependencies(table: &mut Table) {
    if let Some(dependencies) = table
        .remove("dev-dependencies")
        .and_then(|item| item.as_table().cloned())
    {
        if !table.contains_key("dependencies") {
            table["dependencies"] = Item::Table(Table::new());
        }
        if let Some(target) = table["dependencies"].as_table_mut() {
            for (name, dependency) in dependencies {
                if !target.contains_key(&name) {
                    target[&name] = dependency;
                }
            }
        }
    }
    if let Some(targets) = table.get_mut("target").and_then(Item::as_table_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(target) = target.as_table_mut() {
                promote_example_dependencies(target);
            }
        }
    }
}
fn inline_paths(table: &mut InlineTable, base: &Path) {
    for (key, value) in table.iter_mut() {
        if key == "path" {
            if let Some(path) = value.as_str().filter(|path| Path::new(path).is_relative()) {
                *value = Value::from(base.join(path).to_string_lossy().as_ref());
            }
        } else if let Some(child) = value.as_inline_table_mut() {
            inline_paths(child, base);
        }
    }
}
