//! User directories that stay correct under `sudo`: config and backups always
//! belong to the real user, not to root.

use std::ffi::{CStr, CString};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// `(uid, gid)` of the user who invoked sudo, when running as root through sudo.
fn sudo_ids() -> Option<(u32, u32)> {
    if !is_root() {
        return None;
    }
    let uid = std::env::var("SUDO_UID").ok()?.parse().ok()?;
    let gid = std::env::var("SUDO_GID").ok()?.parse().ok()?;
    Some((uid, gid))
}

fn home_of(user: &str) -> Option<PathBuf> {
    let name = CString::new(user).ok()?;
    let pw = unsafe { libc::getpwnam(name.as_ptr()) };
    if pw.is_null() {
        return None;
    }
    let dir = unsafe { CStr::from_ptr((*pw).pw_dir) };
    Some(PathBuf::from(dir.to_str().ok()?))
}

fn home() -> PathBuf {
    if sudo_ids().is_some()
        && let Some(home) = std::env::var("SUDO_USER").ok().and_then(|u| home_of(&u))
    {
        return home;
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    let base = std::env::var_os(var)
        .filter(|v| !v.is_empty() && sudo_ids().is_none())
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(fallback));
    base.join(APP_NAME)
}

pub fn config_file() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("config.toml")
}

pub fn backups_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("backups")
}

/// Under sudo, hands files we create back to the real user.
pub fn fix_owner(path: &Path) {
    if let Some((uid, gid)) = sudo_ids() {
        let _ = std::os::unix::fs::chown(path, Some(uid), Some(gid));
    }
}

pub fn copy_owner(meta: &fs::Metadata, path: &Path) {
    if is_root() {
        let _ = std::os::unix::fs::chown(path, Some(meta.uid()), Some(meta.gid()));
    }
}

/// `create_dir_all` that fixes ownership of every directory it created.
pub fn create_dir_all_owned(dir: &Path) -> std::io::Result<()> {
    let missing: Vec<PathBuf> = dir
        .ancestors()
        .take_while(|p| !p.exists())
        .map(Path::to_path_buf)
        .collect();
    fs::create_dir_all(dir)?;
    for p in missing {
        fix_owner(&p);
    }
    Ok(())
}
