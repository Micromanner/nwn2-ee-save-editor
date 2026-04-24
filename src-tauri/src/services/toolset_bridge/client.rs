use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio};

use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::value::RawValue;
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
    #[error("bridge returned error: {0}")]
    ServeError(String),
    #[error("bridge process died: {0}")]
    ProcessDied(String),
    #[error("failed to parse bridge json: {0}")]
    ParseFailed(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

/// Envelope returned by the serve loop: `{"ok":true,"data":<payload>}` on success,
/// `{"ok":false,"error":"..."}` on failure. `data` stays as a `RawValue` so callers
/// can cache the payload JSON verbatim without a parse+reserialize round-trip.
#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Option<Box<RawValue>>,
    #[serde(default)]
    error: Option<String>,
}

/// Long-lived bridge process. One `serve` child is spawned lazily on first
/// request and reused for every subsequent call, amortizing the ~1s of toolset
/// init (ResourceManager + Harmony patches + TLK open) across all modules in
/// a session.
pub struct BridgeClient {
    bridge_exe: PathBuf,
    nwn2_install: PathBuf,
    cache: GraphCache,
    conn: Mutex<Option<ServeConnection>>,
}

struct ServeConnection {
    child: std::process::Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for ServeConnection {
    fn drop(&mut self) {
        // Best-effort graceful shutdown: ask the serve loop to exit, close our write
        // end so it reads EOF even if the write failed, then detach. We don't
        // wait() here because Drop should stay fast; leftover zombies are the OS's
        // problem at this point (the app is typically shutting down too).
        let _ = writeln!(self.stdin, "{{\"op\":\"shutdown\"}}");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
    }
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
            conn: Mutex::new(None),
        })
    }

    /// Parse a `campaign.cam` and return the ordered module list plus `start_module`.
    /// Accepts either the campaign folder or the `campaign.cam` file itself.
    pub fn list_modules(&self, campaign_path: &Path) -> BridgeResult<CampaignModules> {
        if !campaign_path.exists() {
            return Err(BridgeError::InputNotFound(campaign_path.to_path_buf()));
        }
        let req = serde_json::json!({
            "op": "list_modules",
            "campaign": campaign_path.to_string_lossy(),
        });
        let data = self.request(&req)?;
        Ok(serde_json::from_str(data.get())?)
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
        let req = serde_json::json!({
            "op": "graph",
            "module": module_path.to_string_lossy(),
        });
        let data = self.request(&req)?;
        let raw = data.get();

        if let Err(e) = self.cache.store(&key, raw) {
            warn!("failed to write bridge graph cache: {e}");
        }
        Ok(serde_json::from_str(raw)?)
    }

    /// Send one request line, read one response line. Retries once after
    /// respawning the child if the pipe looks broken — covers the case where
    /// the previous session's child exited between calls.
    fn request(&self, op: &serde_json::Value) -> BridgeResult<Box<RawValue>> {
        let mut guard = self.conn.lock();

        match self.request_on_guard(&mut guard, op) {
            Ok(data) => Ok(data),
            Err(BridgeError::ProcessDied(msg)) => {
                warn!("bridge process died ({msg}); respawning and retrying");
                *guard = None;
                self.request_on_guard(&mut guard, op)
            }
            Err(e) => Err(e),
        }
    }

    fn request_on_guard(
        &self,
        guard: &mut parking_lot::MutexGuard<'_, Option<ServeConnection>>,
        op: &serde_json::Value,
    ) -> BridgeResult<Box<RawValue>> {
        if guard.is_none() {
            **guard = Some(self.spawn_serve()?);
        }
        let conn = guard.as_mut().expect("connection just ensured");

        let mut req_line = serde_json::to_vec(op)?;
        req_line.push(b'\n');
        if let Err(e) = conn
            .stdin
            .write_all(&req_line)
            .and_then(|()| conn.stdin.flush())
        {
            return Err(BridgeError::ProcessDied(format!("write failed: {e}")));
        }

        let mut resp = String::new();
        let n = conn
            .stdout
            .read_line(&mut resp)
            .map_err(|e| BridgeError::ProcessDied(format!("read failed: {e}")))?;
        if n == 0 {
            return Err(BridgeError::ProcessDied("stdout closed (EOF)".to_string()));
        }

        let env: Envelope = serde_json::from_str(&resp)?;
        if env.ok {
            env.data.ok_or_else(|| {
                BridgeError::ServeError("ok=true response missing 'data' field".to_string())
            })
        } else {
            Err(BridgeError::ServeError(
                env.error
                    .unwrap_or_else(|| "unknown serve error".to_string()),
            ))
        }
    }

    fn spawn_serve(&self) -> BridgeResult<ServeConnection> {
        let mut cmd = Command::new(&self.bridge_exe);
        cmd.arg("--nwn2-install")
            .arg(&self.nwn2_install)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(?cmd, "spawning bridge serve process");
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin just set");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout just set"));
        let stderr = child.stderr.take().expect("piped stderr just set");

        spawn_stderr_forwarder(stderr);

        Ok(ServeConnection {
            child,
            stdin,
            stdout,
        })
    }
}

/// Read bridge stderr line-by-line and forward through tracing. Runs on a
/// dedicated thread for the child's lifetime; terminates when the pipe closes.
fn spawn_stderr_forwarder(stderr: ChildStderr) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) if line.is_empty() => {}
                Ok(line) => forward_bridge_line(&line),
                Err(e) => {
                    debug!("bridge stderr reader exiting: {e}");
                    break;
                }
            }
        }
    });
}

/// Route each bridge stderr line through `tracing` so `[timing]` breakdowns and
/// `[info]`/`[warn]`/`[error]` messages land in the app log. Unprefixed lines go
/// out at debug level so they never disappear silently.
fn forward_bridge_line(line: &str) {
    if let Some(rest) = line.strip_prefix("[timing] ") {
        tracing::info!(target: "toolset_bridge", "{rest}");
    } else if let Some(rest) = line.strip_prefix("[info] ") {
        debug!(target: "toolset_bridge", "{rest}");
    } else if let Some(rest) = line.strip_prefix("[warn] ") {
        warn!(target: "toolset_bridge", "{rest}");
    } else if let Some(rest) = line.strip_prefix("[error] ") {
        tracing::error!(target: "toolset_bridge", "{rest}");
    } else {
        debug!(target: "toolset_bridge", "{line}");
    }
}
