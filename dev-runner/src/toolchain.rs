use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// The compiler and runtime protocol version tested together by the plugin.
pub const VERSION: &str = "0.7.10";

/// Resolves a verified compiler from the plugin cache, downloading it on first use.
pub fn ensure(cache: &Path) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CRANPOSE_DX") {
        let path = PathBuf::from(path);
        check_version(&path)?;
        return Ok(path);
    }
    let (target, expected) = distribution(std::env::consts::OS, std::env::consts::ARCH)?;
    let executable = if cfg!(windows) { "dx.exe" } else { "dx" };
    let directory = cache.join(format!("dioxus-{VERSION}-{target}"));
    let path = directory.join(executable);
    if path.is_file() {
        check_version(&path)?;
        return Ok(path);
    }
    fs::create_dir_all(&directory)?;
    let url = format!(
        "https://github.com/DioxusLabs/dioxus/releases/download/v{VERSION}/dx-{target}.zip"
    );
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?
        .get(url)
        .send()?
        .error_for_status()?;
    let mut bytes = Vec::new();
    response.take(256 * 1024 * 1024).read_to_end(&mut bytes)?;
    verify_archive(&bytes, expected)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let index = (0..archive.len())
        .find(|index| {
            archive.by_index(*index).is_ok_and(|entry| {
                entry
                    .enclosed_name()
                    .is_some_and(|name| name.file_name().is_some_and(|name| name == executable))
            })
        })
        .context("compiler executable missing from release archive")?;
    cranpose_plugin_cache::publish_executable(&path, archive.by_index(index)?, check_version)?;
    Ok(path)
}

fn check_version(path: &Path) -> Result<()> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .context("start the hot-patch compiler")?;
    let version = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || version.split_whitespace().nth(1) != Some(VERSION) {
        bail!("expected Dioxus CLI {VERSION}, received {}", version.trim());
    }
    Ok(())
}

fn verify_archive(bytes: &[u8], expected: &str) -> Result<()> {
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual != expected {
        bail!("hot-patch compiler archive checksum mismatch");
    }
    Ok(())
}

fn distribution(os: &str, arch: &str) -> Result<(&'static str, &'static str)> {
    Ok(match (os, arch) {
        ("macos", "aarch64") => (
            "aarch64-apple-darwin",
            "4aff37aea6b8fbce7bee3f8bbbea1bd71f60e0fa09537f35759f6494da861e37",
        ),
        ("macos", "x86_64") => (
            "x86_64-apple-darwin",
            "b2751a700b8b02f85fcdd8fe6e63c20876b4af9f0921c5be4c703e6f872b67ee",
        ),
        ("linux", "aarch64") => (
            "aarch64-unknown-linux-gnu",
            "16985f1e1f88fbcf48f3a3cbdbb0c10c9673d0846d88cbc43783b92fd60c8c95",
        ),
        ("linux", "x86_64") => (
            "x86_64-unknown-linux-gnu",
            "61d2132eaf3cd70ca1ac89a7ecc891fcd770e917a5869e29d465459371b26c83",
        ),
        ("windows", "aarch64") => (
            "aarch64-pc-windows-msvc",
            "77e807cc7e5e036b3283186efd13fc2c2501f085ef93aaf817fd1728392f29a3",
        ),
        ("windows", "x86_64") => (
            "x86_64-pc-windows-msvc",
            "45eb4f87b7f86fdba8508ee7e0ef1f9e13cf8f2ffcc6a2d5858d0e8ea7dd1531",
        ),
        _ => bail!("hot reload is not available on {os}/{arch}"),
    })
}

#[cfg(test)]
#[path = "../tests/unit/toolchain.rs"]
mod tests;
