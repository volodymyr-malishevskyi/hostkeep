use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::paths;

pub const BACKUP_PREFIX: &str = "hosts-";

/// Saves a copy of `content` into `dir` and prunes all but the newest `keep` backups.
pub fn backup(content: &str, dir: &Path, keep: usize) -> Result<PathBuf> {
    paths::create_dir_all_owned(dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    // Several saves within one second get `-2`, `-3`… after the newest existing one.
    let next = list_backups(dir)
        .iter()
        .map(|p| backup_key(p))
        .filter(|(s, _)| *s == stamp)
        .map(|(_, n)| n + 1)
        .max();
    let path = match next {
        None => dir.join(format!("{BACKUP_PREFIX}{stamp}")),
        Some(n) => dir.join(format!("{BACKUP_PREFIX}{stamp}-{n}")),
    };
    fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    paths::fix_owner(&path);

    let backups = list_backups(dir);
    for old in backups.iter().skip(keep.max(1)) {
        let _ = fs::remove_file(old);
    }
    Ok(path)
}

/// Newest first.
pub fn list_backups(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(BACKUP_PREFIX))
        })
        .collect();
    files.sort_by_key(|p| backup_key(p));
    files.reverse();
    files
}

/// `(timestamp, n)`: timestamps sort lexically, `-N` orders saves within the same second.
fn backup_key(path: &Path) -> (String, u32) {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let rest = name.strip_prefix(BACKUP_PREFIX).unwrap_or(name);
    match rest.rsplit_once('-') {
        Some((stamp, n)) if stamp.contains('-') => match n.parse() {
            Ok(n) => (stamp.to_string(), n),
            Err(_) => (rest.to_string(), 1),
        },
        _ => (rest.to_string(), 1),
    }
}

/// Whether the current user can write the file without privilege escalation.
pub fn can_write(path: &Path) -> bool {
    paths::is_root() || fs::OpenOptions::new().append(true).open(path).is_ok()
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriteResult {
    Done,
    /// sudo needs a password: the caller must leave the TUI and call [`write_with_sudo`].
    NeedsPassword,
}

/// Writes `content` to `path`: atomically when we have permission, otherwise via `sudo -n`.
pub fn write(path: &Path, content: &str) -> Result<WriteResult> {
    match write_atomic(path, content) {
        Ok(()) => Ok(WriteResult::Done),
        Err(e) if is_permission_denied(&e) => {
            let staged = stage(content)?;
            let ok = sudo_copy(&staged, path, false)?;
            let _ = fs::remove_file(&staged);
            Ok(if ok {
                WriteResult::Done
            } else {
                WriteResult::NeedsPassword
            })
        }
        Err(e) => Err(e),
    }
}

/// Interactive variant: sudo may prompt in the terminal, so the TUI must be suspended.
pub fn write_with_sudo(path: &Path, content: &str) -> Result<()> {
    let staged = stage(content)?;
    let ok = sudo_copy(&staged, path, true);
    let _ = fs::remove_file(&staged);
    if !ok? {
        bail!("sudo failed");
    }
    Ok(())
}

fn is_permission_denied(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<std::io::Error>())
        .any(|io| io.kind() == ErrorKind::PermissionDenied || io.raw_os_error() == Some(libc::EROFS))
}

/// Temp file in the same directory, then rename: readers never see a half-written file.
fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let meta = fs::metadata(path).ok();
    let mut tmp = tempfile::Builder::new().prefix(".hostkeep-").tempfile_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    tmp.as_file().sync_all()?;
    if let Some(meta) = &meta {
        fs::set_permissions(tmp.path(), meta.permissions())?;
        paths::copy_owner(meta, tmp.path());
    }
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn stage(content: &str) -> Result<PathBuf> {
    let mut tmp = tempfile::Builder::new().prefix("hostkeep-").tempfile()?;
    tmp.write_all(content.as_bytes())?;
    let (_, path) = tmp.keep()?;
    Ok(path)
}

/// `cp` into the existing file keeps its owner and mode.
fn sudo_copy(from: &Path, to: &Path, interactive: bool) -> Result<bool> {
    let mut cmd = Command::new("sudo");
    if !interactive {
        cmd.arg("-n").stdin(Stdio::null()).stderr(Stdio::null());
    }
    cmd.arg("cp").arg(from).arg(to);
    if !interactive {
        return Ok(cmd.status().context("run sudo")?.success());
    }
    // The terminal is in cooked mode now, so Ctrl+C at the password prompt would also
    // kill us and lose unsaved changes. Ignore SIGINT here; sudo gets the default back.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            Ok(())
        });
    }
    let previous = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    let status = cmd.status();
    unsafe { libc::signal(libc::SIGINT, previous) };
    Ok(status.context("run sudo")?.success())
}

/// Flushes the OS DNS cache, using `sudo -n` (no prompt) when not root.
/// `None` when the system has no cache to flush (Linux without systemd-resolved).
pub fn flush_dns() -> Option<bool> {
    let script = if cfg!(target_os = "macos") {
        "dscacheutil -flushcache; killall -HUP mDNSResponder"
    } else if Command::new("sh")
        .args(["-c", "command -v resolvectl"])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        "resolvectl flush-caches"
    } else {
        return None;
    };
    let mut cmd = if paths::is_root() {
        Command::new("sh")
    } else {
        let mut c = Command::new("sudo");
        c.args(["-n", "sh"]);
        c
    };
    let status = cmd
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Some(status.is_ok_and(|s| s.success()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_keeps_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hosts");
        fs::write(&file, "old\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(write(&file, "new\n").unwrap(), WriteResult::Done);
        assert_eq!(fs::read_to_string(&file).unwrap(), "new\n");
        assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o644);
    }

    #[test]
    fn backups_are_pruned_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            backup(&format!("v{i}"), dir.path(), 3).unwrap();
        }
        let list = list_backups(dir.path());
        assert_eq!(list.len(), 3);
        assert_eq!(fs::read_to_string(&list[0]).unwrap(), "v4");
        assert_eq!(fs::read_to_string(&list[2]).unwrap(), "v2");
    }
}
