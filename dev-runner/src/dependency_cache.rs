//! Reuse a development lockfile without changing dependency resolution rules.
use anyhow::{Result, ensure};
use cranpose_plugin_cache::DerivedFile;
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

pub struct DependencyCache {
    record: DerivedFile,
    lock: PathBuf,
    pub hit: bool,
}
impl DependencyCache {
    /// The caller supplies its private workspace, after all manifest instrumentation.
    pub fn restore(workspace: &Path, root: &Path) -> Result<Self> {
        let mut inputs = vec![("format".to_owned(), b"cranpose-dev-lock-v1".to_vec())];
        for entry in walkdir::WalkDir::new(workspace) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(workspace)?;
            let name = relative.to_string_lossy().replace('\\', "/");
            if entry.file_name() != "Cargo.toml"
                && !matches!(
                    name.as_str(),
                    "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml"
                )
                && !name.ends_with(".cargo/config")
                && !name.ends_with(".cargo/config.toml")
            {
                continue;
            }
            let bytes = fs::read(entry.path())?;
            // Instrumented internal path dependencies contain the unique session root.
            // Normalize both literal and TOML-escaped Windows paths for the cache key.
            let contents = if name == "Cargo.lock" {
                bytes
            } else {
                normalize(&String::from_utf8(bytes)?, workspace).into_bytes()
            };
            inputs.push((name, contents));
        }
        let entries: Vec<_> = inputs
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        let record = DerivedFile::new(root, &entries)?;
        let lock = workspace.join("Cargo.lock");
        let mut hit = false;
        if let Some(bytes) = record.load()?.filter(|bytes| valid_lock(bytes)) {
            fs::write(&lock, bytes)?;
            hit = true;
        }
        Ok(Self { record, lock, hit })
    }
    /// Call after the compiler has stopped writing. A cancelled first resolution
    /// may have produced no lockfile, so absence is an ordinary cache miss.
    pub fn save(&self) -> Result<()> {
        let bytes = match fs::read(&self.lock) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        ensure!(valid_lock(&bytes), "Incomplete development lockfile");
        self.record.store(&bytes)
    }
}
fn valid_lock(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.parse::<DocumentMut>().ok())
        .is_some_and(|document| {
            document
                .get("package")
                .and_then(|item| item.as_array_of_tables())
                .is_some_and(|packages| {
                    !packages.is_empty()
                        && packages.iter().all(|package| {
                            ["name", "version"].iter().all(|key| {
                                package.get(key).and_then(|value| value.as_str()).is_some()
                            })
                        })
                })
        })
}
fn normalize(text: &str, workspace: &Path) -> String {
    let root = workspace.to_string_lossy();
    text.replace(&root.replace('\\', "\\\\"), "$CRANPOSE_WORKSPACE")
        .replace(root.as_ref(), "$CRANPOSE_WORKSPACE")
}

#[cfg(test)]
mod tests {
    use super::*;
    const ORIGINAL: &str = "version = 4\n[[package]]\nname = 'app'\nversion = '1.0.0'\n";
    const RESOLVED: &str = "version = 4\n[[package]]\nname = 'app'\nversion = '1.0.0'\n[[package]]\nname = 'dev-helper'\nversion = '1.0.0'\n";
    fn fixture(path: &Path) {
        fs::create_dir_all(path).expect("dependency cache fixture operation");
        fs::write(
            path.join("Cargo.toml"),
            format!(
                "[package]\nname='app'\nversion='1.0.0'\n[dependencies.member]\npath={:?}\n",
                path.join("member")
            ),
        )
        .expect("dependency cache fixture operation");
        fs::write(path.join("Cargo.lock"), ORIGINAL).expect("dependency cache fixture operation");
    }
    #[test]
    fn seeds_survive_session_paths_but_manifest_and_original_lock_edits_invalidate() {
        let temp = tempfile::tempdir().expect("dependency cache fixture operation");
        let root = temp.path().join("cache");
        let first = temp.path().join("first");
        fixture(&first);
        let cache =
            DependencyCache::restore(&first, &root).expect("dependency cache fixture operation");
        assert!(!cache.hit);
        fs::write(first.join("Cargo.lock"), RESOLVED).expect("dependency cache fixture operation");
        cache.save().expect("dependency cache fixture operation");
        let second = temp.path().join("second");
        fixture(&second);
        assert!(
            DependencyCache::restore(&second, &root)
                .expect("dependency cache fixture operation")
                .hit
        );
        assert_eq!(
            fs::read_to_string(second.join("Cargo.lock"))
                .expect("dependency cache fixture operation"),
            RESOLVED
        );
        fixture(&second);
        fs::write(
            second.join("Cargo.lock"),
            ORIGINAL.replace("1.0.0", "2.0.0"),
        )
        .expect("dependency cache fixture operation");
        assert!(
            !DependencyCache::restore(&second, &root)
                .expect("dependency cache fixture operation")
                .hit
        );
        fixture(&second);
        fs::write(
            second.join("Cargo.toml"),
            "[package]\nname='renamed'\nversion='1.0.0'\n",
        )
        .expect("dependency cache fixture operation");
        assert!(
            !DependencyCache::restore(&second, &root)
                .expect("dependency cache fixture operation")
                .hit
        );
    }
    #[test]
    fn missing_or_incomplete_resolution_does_not_publish_a_seed() {
        let temp = tempfile::tempdir().expect("dependency cache fixture operation");
        let workspace = temp.path().join("project");
        fixture(&workspace);
        let cache = DependencyCache::restore(&workspace, &temp.path().join("cache"))
            .expect("dependency cache fixture operation");
        fs::remove_file(workspace.join("Cargo.lock")).expect("dependency cache fixture operation");
        cache.save().expect("dependency cache fixture operation");
        assert!(
            cache
                .record
                .load()
                .expect("dependency cache fixture operation")
                .is_none()
        );
        for partial in ["version = 4\n", "version = 4\n[[package]]\nname = 'app'\n"] {
            fs::write(workspace.join("Cargo.lock"), partial).expect("write incomplete lockfile");
            assert!(cache.save().is_err());
            assert!(cache.record.load().expect("load missing seed").is_none());
        }
    }
    #[test]
    fn windows_literal_and_escaped_roots_normalize_consistently() {
        let path = Path::new(r"C:\cache\session-123");
        assert_eq!(
            normalize(r"C:\cache\session-123\member", path),
            r"$CRANPOSE_WORKSPACE\member"
        );
        assert_eq!(
            normalize(r"C:\\cache\\session-123\\member", path),
            r"$CRANPOSE_WORKSPACE\\member"
        );
    }
}
