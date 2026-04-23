use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tracing::debug;

use super::client::{BridgeClient, BridgeError, BridgeResult};
use crate::state::AppState;

/// Bundle layout (from `tauri.conf.json` `bundle.resources`):
/// `../toolset-bridge/dist/toolset-bridge.exe`. Tauri rewrites `../` paths as
/// `_up_` relative to the resource dir, so the runtime location is:
/// `<resource_dir>/_up_/toolset-bridge/dist/toolset-bridge.exe`.
const BRIDGE_EXE_REL: &str = "_up_/toolset-bridge/dist/toolset-bridge.exe";

/// Cache subdirectory under the user's platform cache dir.
const CACHE_SUBDIR: &str = "nwn2_save_editor/toolset_bridge";

/// Build a `BridgeClient` wired to bundled resources and the user's NWN2 install.
///
/// Resolution order:
/// - **Bridge exe**: `app.path().resource_dir()? / _up_/toolset-bridge/dist/toolset-bridge.exe`
/// - **NWN2 install**: `state.paths.game_folder()` — errors if the user hasn't set one
/// - **Cache dir**: `dirs::cache_dir() / nwn2_save_editor/toolset_bridge`, falling back to the OS temp dir
pub fn build_client(app: &AppHandle, state: &AppState) -> BridgeResult<BridgeClient> {
    if !cfg!(windows) {
        // Linux (Wine/Proton) invocation is deferred (NWN-129). The `dist/*` files
        // still ship inside AppImage/Deb as a ~1.5 MB ride-along, but runtime is gated off.
        return Err(BridgeError::PlatformUnsupported(
            "toolset bridge runtime is Windows-only (Linux via Wine/Proton deferred)".to_string(),
        ));
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| BridgeError::Io(std::io::Error::other(e)))?;
    let bridge_exe = resource_dir.join(BRIDGE_EXE_REL);

    let nwn2_install = state
        .paths
        .read()
        .game_folder()
        .cloned()
        .ok_or_else(|| BridgeError::InstallNotFound(PathBuf::from("<unset>")))?;

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(CACHE_SUBDIR);

    debug!(
        bridge_exe = %bridge_exe.display(),
        nwn2_install = %nwn2_install.display(),
        cache_dir = %cache_dir.display(),
        "building toolset bridge client"
    );

    BridgeClient::new(bridge_exe, nwn2_install, cache_dir)
}
