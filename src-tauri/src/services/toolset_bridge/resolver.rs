use std::sync::Arc;

use tauri::AppHandle;

use super::client::{BridgeClient, BridgeError, BridgeResult};
use crate::state::AppState;

/// Cache subdirectory under the user's platform cache dir.
#[cfg(windows)]
const CACHE_SUBDIR: &str = "nwn2_save_editor/toolset_bridge";

/// Return the shared long-lived `BridgeClient`, constructing it on first call.
/// The client owns a persistent `serve` subprocess whose toolset init is paid
/// once — reused across every subsequent Tauri command in this session.
///
/// Resolution order for the initial construction:
/// - **Bridge exe**: embedded in this binary at compile time and extracted to
///   `<cache dir>/bin/` (see [`embedded`]), so the editor exe is fully portable
/// - **NWN2 install**: `state.paths.game_folder()` — errors if the user hasn't set one
/// - **Cache dir**: `dirs::cache_dir() / nwn2_save_editor/toolset_bridge`, falling back to the OS temp dir
pub fn build_client(_app: &AppHandle, state: &AppState) -> BridgeResult<Arc<BridgeClient>> {
    if let Some(cached) = state.bridge_client.lock().as_ref().cloned() {
        return Ok(cached);
    }

    #[cfg(not(windows))]
    {
        // Linux (Wine/Proton) invocation is deferred (NWN-129).
        Err(BridgeError::PlatformUnsupported(
            "toolset bridge runtime is Windows-only (Linux via Wine/Proton deferred)".to_string(),
        ))
    }

    #[cfg(windows)]
    {
        let nwn2_install = state
            .paths
            .read()
            .game_folder()
            .cloned()
            .ok_or_else(|| BridgeError::InstallNotFound(std::path::PathBuf::from("<unset>")))?;

        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(CACHE_SUBDIR);

        let bridge_exe = embedded::ensure_extracted(&cache_dir)?;

        tracing::debug!(
            bridge_exe = %bridge_exe.display(),
            nwn2_install = %nwn2_install.display(),
            cache_dir = %cache_dir.display(),
            "building toolset bridge client"
        );

        let client = Arc::new(BridgeClient::new(bridge_exe, nwn2_install, cache_dir)?);

        // Double-check the slot under the write lock in case a concurrent caller
        // raced us to construction; last writer wins but both see the same Arc.
        let mut slot = state.bridge_client.lock();
        if let Some(existing) = slot.as_ref() {
            return Ok(existing.clone());
        }
        *slot = Some(client.clone());
        Ok(client)
    }
}

/// The bridge and its .NET dependencies are baked into the editor binary so a
/// lone `nwn2ee-save-editor.exe` works from any folder. `CreateProcess` and the
/// CLR need real files, so they are written once to `<cache dir>/bin/` and
/// refreshed whenever the embedded payload changes (tracked by a SHA-256 stamp).
#[cfg(windows)]
mod embedded {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use sha2::{Digest, Sha256};
    use tracing::info;

    use super::super::client::BridgeResult;

    const BRIDGE_FILES: &[(&str, &[u8])] = &[
        (
            "toolset-bridge.exe",
            include_bytes!("../../../../toolset-bridge/dist/toolset-bridge.exe"),
        ),
        (
            "toolset-bridge.exe.config",
            include_bytes!("../../../../toolset-bridge/dist/toolset-bridge.exe.config"),
        ),
    ];

    const BIN_SUBDIR: &str = "bin";
    const STAMP_FILE: &str = ".bridge_hash";

    fn payload_hash() -> &'static str {
        static HASH: OnceLock<String> = OnceLock::new();
        HASH.get_or_init(|| {
            let mut h = Sha256::new();
            for (name, bytes) in BRIDGE_FILES {
                h.update(name.as_bytes());
                h.update(bytes);
            }
            hex::encode(h.finalize())
        })
    }

    /// Return the path of the extracted `toolset-bridge.exe`, writing the payload
    /// out first if the stamp is missing, stale, or any file is absent. The stamp
    /// is written last so an interrupted extraction is retried on the next launch.
    pub fn ensure_extracted(cache_dir: &Path) -> BridgeResult<PathBuf> {
        let bin_dir = cache_dir.join(BIN_SUBDIR);
        let stamp_path = bin_dir.join(STAMP_FILE);
        let hash = payload_hash();

        let up_to_date = fs::read_to_string(&stamp_path).is_ok_and(|s| s == hash)
            && BRIDGE_FILES
                .iter()
                .all(|(name, _)| bin_dir.join(name).is_file());

        if !up_to_date {
            info!(dir = %bin_dir.display(), "extracting embedded toolset bridge");
            fs::create_dir_all(&bin_dir)?;
            for (name, bytes) in BRIDGE_FILES {
                let tmp = bin_dir.join(format!("{name}.tmp"));
                fs::write(&tmp, bytes)?;
                fs::rename(&tmp, bin_dir.join(name))?;
            }
            fs::write(&stamp_path, hash)?;
        }

        Ok(bin_dir.join(BRIDGE_FILES[0].0))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn extracts_once_and_refreshes_on_stale_stamp() {
            let dir = tempfile::tempdir().unwrap();
            let exe = ensure_extracted(dir.path()).unwrap();
            let bin = dir.path().join(BIN_SUBDIR);
            assert_eq!(exe, bin.join("toolset-bridge.exe"));
            for (name, bytes) in BRIDGE_FILES {
                assert_eq!(fs::read(bin.join(name)).unwrap(), *bytes);
                assert!(!bin.join(format!("{name}.tmp")).exists());
            }
            assert_eq!(
                fs::read_to_string(bin.join(STAMP_FILE)).unwrap(),
                payload_hash()
            );

            let before = fs::metadata(&exe).unwrap().modified().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            ensure_extracted(dir.path()).unwrap();
            assert_eq!(fs::metadata(&exe).unwrap().modified().unwrap(), before);

            fs::write(bin.join(STAMP_FILE), "stale").unwrap();
            fs::write(&exe, b"garbage").unwrap();
            ensure_extracted(dir.path()).unwrap();
            assert_eq!(fs::read(&exe).unwrap(), BRIDGE_FILES[0].1);
        }
    }
}
