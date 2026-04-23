use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;
use tracing::{debug, info, warn};

use super::cache::GraphCache;
use super::types::{CampaignModules, ModuleGraph};

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("bridge executable not found: {0}")]
    BridgeNotFound(PathBuf),
    #[error("NWN2 install not found: {0}")]
    InstallNotFound(PathBuf),
    #[error("input path not found: {0}")]
    InputNotFound(PathBuf),
    #[error("toolset bridge is not available on this platform: {0}")]
    PlatformUnsupported(String),
    #[error("bridge exited with code {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },
    #[error("bridge terminated by signal: {stderr}")]
    KilledBySignal { stderr: String },
    #[error("failed to parse bridge json: {0}")]
    ParseFailed(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

pub struct BridgeClient {
    bridge_exe: PathBuf,
    nwn2_install: PathBuf,
    cache: GraphCache,
}

impl BridgeClient {
    pub fn new(
        bridge_exe: PathBuf,
        nwn2_install: PathBuf,
        cache_dir: PathBuf,
    ) -> BridgeResult<Self> {
        if !bridge_exe.is_file() {
            return Err(BridgeError::BridgeNotFound(bridge_exe));
        }
        if !nwn2_install.is_dir() {
            return Err(BridgeError::InstallNotFound(nwn2_install));
        }
        let cache = GraphCache::new(cache_dir, &bridge_exe)?;
        Ok(Self {
            bridge_exe,
            nwn2_install,
            cache,
        })
    }

    /// Parse a `campaign.cam` and return the ordered module list plus `start_module`.
    /// Accepts either the campaign folder or the `campaign.cam` file itself.
    pub fn list_modules(&self, campaign_path: &Path) -> BridgeResult<CampaignModules> {
        if !campaign_path.exists() {
            return Err(BridgeError::InputNotFound(campaign_path.to_path_buf()));
        }
        let out = self.run_capture(&[
            "--campaign",
            &campaign_path.to_string_lossy(),
            "list-modules",
        ])?;
        Ok(serde_json::from_str(&out)?)
    }

    /// Extract the full reference graph (journal, factions, convo, module variables)
    /// for a single `.mod` file. Results are cached on disk by module-file fingerprint.
    pub fn graph(&self, module_path: &Path) -> BridgeResult<ModuleGraph> {
        if !module_path.exists() {
            return Err(BridgeError::InputNotFound(module_path.to_path_buf()));
        }

        let key = self.cache.key(module_path)?;
        if let Some(cached) = self.cache.load(&key) {
            debug!(?module_path, key = %key, "bridge graph cache hit");
            return Ok(serde_json::from_str(&cached)?);
        }

        info!(?module_path, "bridge graph cache miss; invoking bridge");
        let out = self.run_capture(&["--module", &module_path.to_string_lossy(), "graph"])?;

        if let Err(e) = self.cache.store(&key, &out) {
            warn!("failed to write bridge graph cache: {e}");
        }
        Ok(serde_json::from_str(&out)?)
    }

    fn run_capture(&self, extra: &[&str]) -> BridgeResult<String> {
        let mut cmd = Command::new(&self.bridge_exe);
        cmd.arg("--nwn2-install")
            .arg(&self.nwn2_install)
            .args(extra);
        // Bridge writes JSON to stdout when --out is "-" (the default).

        debug!(?cmd, "spawning bridge");
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return match output.status.code() {
                Some(c) => Err(BridgeError::NonZeroExit { code: c, stderr }),
                None => Err(BridgeError::KilledBySignal { stderr }),
            };
        }

        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            BridgeError::ParseFailed(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("bridge stdout was not UTF-8: {e}"),
            )))
        })?;
        Ok(stdout)
    }
}
