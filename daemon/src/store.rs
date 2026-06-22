//! On-disk storage of tunnel definitions (one JSON file per tunnel).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::WgConfig;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no tunnel with id {0}")]
    NotFound(String),
    #[error("could not allocate a unique tunnel id")]
    NoUniqueId,
}

/// A stored tunnel: a stable id, a display name, and its WireGuard config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub config: WgConfig,
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn read_file(path: &Path) -> Result<Tunnel, StoreError> {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }

    /// All tunnels, sorted by display name. Unreadable files are skipped.
    pub fn list(&self) -> Result<Vec<Tunnel>, StoreError> {
        let mut tunnels = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match Self::read_file(&path) {
                Ok(t) => tunnels.push(t),
                Err(e) => eprintln!("wirearchd: skipping {}: {e}", path.display()),
            }
        }
        tunnels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(tunnels)
    }

    pub fn get(&self, id: &str) -> Result<Tunnel, StoreError> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        Self::read_file(&path)
    }

    /// Persist a tunnel atomically (write a temp file, then rename).
    pub fn save(&self, tunnel: &Tunnel) -> Result<(), StoreError> {
        let data = serde_json::to_string_pretty(tunnel)?;
        let path = self.path_for(&tunnel.id);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, data)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn remove(&self, id: &str) -> Result<(), StoreError> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    /// A unique, filesystem-safe id derived from a display name.
    pub fn unique_id(&self, name: &str) -> Result<String, StoreError> {
        let base = match slugify(name) {
            s if s.is_empty() => "tunnel".to_string(),
            s => s,
        };
        if !self.path_for(&base).exists() {
            return Ok(base);
        }
        for n in 2..10_000 {
            let candidate = format!("{base}-{n}");
            if !self.path_for(&candidate).exists() {
                return Ok(candidate);
            }
        }
        Err(StoreError::NoUniqueId)
    }
}

/// Lowercase, collapse non-alphanumerics to single hyphens, trim hyphens.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hetzner DE"), "hetzner-de");
        assert_eq!(slugify("  weird__name!! "), "weird-name");
        assert_eq!(slugify("///"), "");
        assert_eq!(slugify("turhost"), "turhost");
    }
}
