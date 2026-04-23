use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// On-disk cache for bridge `graph` output. Keyed by the module file's absolute path,
/// mtime, and size. The bridge binary's own mtime is mixed into the key so that
/// rebuilding the bridge invalidates the cache automatically.
pub(super) struct GraphCache {
    dir: PathBuf,
    bridge_mtime_ns: i128,
}

impl GraphCache {
    pub fn new(cache_dir: PathBuf, bridge_exe: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        let bridge_mtime_ns = fs::metadata(bridge_exe)?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(0);
        Ok(Self {
            dir: cache_dir,
            bridge_mtime_ns,
        })
    }

    pub fn key(&self, module_path: &Path) -> std::io::Result<String> {
        let meta = fs::metadata(module_path)?;
        let mtime_ns = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(0);
        let size = meta.len();

        let mut h = Sha256::new();
        h.update(module_path.to_string_lossy().as_bytes());
        h.update(mtime_ns.to_le_bytes());
        h.update(size.to_le_bytes());
        h.update(self.bridge_mtime_ns.to_le_bytes());
        Ok(format!("{:x}", h.finalize()))
    }

    pub fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.graph.json"))
    }

    pub fn load(&self, key: &str) -> Option<String> {
        fs::read_to_string(self.path(key)).ok()
    }

    pub fn store(&self, key: &str, json: &str) -> std::io::Result<()> {
        fs::write(self.path(key), json)
    }
}
