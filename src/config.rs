use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths;

pub const SYSTEM_HOSTS: &str = "/etc/hosts";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub hosts_path: PathBuf,
    pub backups_keep: usize,
    pub flush_dns: bool,
    /// `auto`, `uk` or `en`.
    pub language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hosts_path: PathBuf::from(SYSTEM_HOSTS),
            backups_keep: 20,
            flush_dns: true,
            language: "auto".into(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).with_context(|| format!("parse {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            paths::create_dir_all_owned(dir)?;
        }
        fs::write(path, toml::to_string_pretty(self)?).with_context(|| format!("write {}", path.display()))?;
        paths::fix_owner(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_and_partial_file_merges() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(Config::load(&path).unwrap(), Config::default());

        fs::write(&path, "language = \"uk\"\n").unwrap();
        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.language, "uk");
        assert_eq!(cfg.backups_keep, 20);

        let cfg = Config {
            flush_dns: false,
            ..cfg
        };
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
    }
}
