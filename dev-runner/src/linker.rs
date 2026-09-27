//! Linux adapter for Dioxus 0.7.10's empty fat-archive cache entries.
//! With a Cargo target outside the private workspace, dx can cache a zero-byte
//! libdeps archive, then pass it to cc on the next launch. Ignore only that empty
//! owned input; retain its separately listed rlibs and every other linker argument.
use anyhow::{Context, Result};
use cranpose_plugin_cache::WorkspaceLease;
use std::{
    ffi::{OsStr, OsString},
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};

const PROXY: &str = "CRANPOSE_LINKER_PROXY";
const REAL: &str = "CRANPOSE_LINKER_REAL_CC";
const ORIGINAL_PATH: &str = "CRANPOSE_LINKER_ORIGINAL_PATH";
const CACHE: &str = "CRANPOSE_LINKER_CACHE";

pub(crate) fn configure(
    command: &mut Command,
    lease: Option<&WorkspaceLease>,
    workspace: &Path,
    target: &Path,
) -> Result<()> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let real = std::env::split_paths(&path)
        .map(|directory| directory.join("cc"))
        .find(|candidate| {
            fs::metadata(candidate).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
        .context("Linux hot reload needs a C compiler (cc) on PATH")?
        .canonicalize()?;
    let source = std::env::current_exe()?;
    let proxy = if let Some(lease) = lease {
        lease.stage_executable_as(&source, OsStr::new("cc"))?
    } else {
        let destination = workspace.join(".cranpose-native-tools/cc");
        cranpose_plugin_cache::publish_executable(&destination, fs::File::open(source)?, |_| {
            Ok(())
        })?;
        destination.canonicalize()?
    };
    fs::create_dir_all(target)?;
    let proxy_directory = proxy.parent().context("linker proxy directory")?;
    command
        .env(PROXY, &proxy)
        .env(REAL, real)
        .env(ORIGINAL_PATH, &path)
        .env(CACHE, target.canonicalize()?)
        .env(
            "PATH",
            std::env::join_paths(
                std::iter::once(proxy_directory.to_owned()).chain(std::env::split_paths(&path)),
            )?,
        );
    Ok(())
}

/// Runs only when this native executable was invoked through its private cc alias.
/// exec preserves the compiler's exit status and process ownership without a shell.
pub fn dispatch() -> Result<()> {
    let Some(proxy) = std::env::var_os(PROXY) else {
        return Ok(());
    };
    if std::env::current_exe()? != PathBuf::from(proxy) {
        return Ok(());
    }
    let real = std::env::var_os(REAL).context("preview linker command")?;
    let path = std::env::var_os(ORIGINAL_PATH).context("preview linker PATH")?;
    let cache = PathBuf::from(std::env::var_os(CACHE).context("preview linker cache")?);
    let arguments = filter_empty_archives(std::env::args_os().skip(1).collect(), &cache);
    let error = Command::new(real)
        .args(arguments)
        .env("PATH", path)
        .env_remove(PROXY)
        .exec();
    Err(error).context("execute preview C compiler")
}

fn filter_empty_archives(arguments: Vec<OsString>, cache: &Path) -> Vec<OsString> {
    arguments
        .iter()
        .enumerate()
        .filter(|(index, argument)| {
            let index = *index;
            !(index > 0
                && arguments[index - 1] == "-Wl,--whole-archive"
                && arguments
                    .get(index + 1)
                    .is_some_and(|next| next == "-Wl,--no-whole-archive")
                && empty_owned_archive(Path::new(argument), cache))
        })
        .map(|(_, argument)| argument.clone())
        .collect()
}

fn empty_owned_archive(path: &Path, cache: &Path) -> bool {
    let Some(hash) = path
        .file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| name.strip_prefix("libdeps-"))
        .and_then(|name| name.strip_suffix(".a"))
    else {
        return false;
    };
    hash.len() == 8
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && fs::symlink_metadata(path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == 0)
        && path
            .canonicalize()
            .is_ok_and(|path| path.starts_with(cache))
}

#[cfg(test)]
#[path = "../tests/unit/linker.rs"]
mod tests;
