use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
/// init (ResourceManager + GUI stub detour + TLK open) across all modules in
/// a session.
pub struct BridgeClient {
    bridge_exe: PathBuf,
    nwn2_install: PathBuf,
    cache: GraphCache,
    conn: Mutex<Option<ServeConnection>>,
}

struct ServeConnection {
    child: Option<std::process::Child>,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// How long a serve process gets to exit on its own before it is killed.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

impl Drop for ServeConnection {
    fn drop(&mut self) {
        // Ask the serve loop to exit on its own; a detached thread kills it only if it
        // is still running after the grace period, so dropping never blocks the caller.
        let _ = writeln!(self.stdin, "{{\"op\":\"shutdown\"}}");
        let _ = self.stdin.flush();
        let Some(mut child) = self.child.take() else {
            return;
        };
        std::thread::spawn(move || {
            let deadline = Instant::now() + SHUTDOWN_GRACE;
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Ok(Some(_)) | Err(_) => return,
                }
            }
            let _ = child.kill();
        });
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
        self.graph_on(module_path, &mut self.conn.lock())
    }

    /// [`Self::graph`] for several modules, results in input order, spread over up
    /// to [`parallel_workers`] threads. The first uses the shared serve process; the
    /// others start their own on their first cache miss and shut it down when the
    /// batch is done, so the extra memory (~0.5 GB each) is only held during a cold
    /// load. `on_done` is called on the calling thread with the number finished.
    pub fn graphs(
        &self,
        module_paths: &[&Path],
        on_done: &mut dyn FnMut(usize),
    ) -> Vec<BridgeResult<ModuleGraph>> {
        let mut results: Vec<Option<BridgeResult<ModuleGraph>>> =
            module_paths.iter().map(|_| None).collect();
        let next = AtomicUsize::new(0);
        let (tx, rx) = mpsc::channel();
        std::thread::scope(|scope| {
            for worker in 0..parallel_workers().min(module_paths.len()) {
                let (tx, next) = (tx.clone(), &next);
                scope.spawn(move || {
                    let mut shared;
                    let mut own = None;
                    let slot = if worker == 0 {
                        shared = self.conn.lock();
                        &mut *shared
                    } else {
                        &mut own
                    };
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = module_paths.get(i) else {
                            break;
                        };
                        if tx.send((i, self.graph_on(path, slot))).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx);
            for (done, (i, result)) in rx.into_iter().enumerate() {
                results[i] = Some(result);
                on_done(done + 1);
            }
        });

        results
            .into_iter()
            .map(|r| r.expect("every module fetched"))
            .collect()
    }

    /// Cached graph, or a bridge run on `slot` (spawned on first use) for a miss.
    fn graph_on(
        &self,
        module_path: &Path,
        slot: &mut Option<ServeConnection>,
    ) -> BridgeResult<ModuleGraph> {
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
        let data = self.request_on(slot, &req)?;
        let raw = data.get();

        if let Err(e) = self.cache.store(&key, raw) {
            warn!("failed to write bridge graph cache: {e}");
        }
        Ok(serde_json::from_str(raw)?)
    }

    fn request(&self, op: &serde_json::Value) -> BridgeResult<Box<RawValue>> {
        self.request_on(&mut self.conn.lock(), op)
    }

    /// Send one request line, read one response line. Retries once after
    /// respawning the child if the pipe looks broken — covers the case where
    /// the previous session's child exited between calls.
    fn request_on(
        &self,
        slot: &mut Option<ServeConnection>,
        op: &serde_json::Value,
    ) -> BridgeResult<Box<RawValue>> {
        match self.request_once(slot, op) {
            Ok(data) => Ok(data),
            Err(BridgeError::ProcessDied(msg)) => {
                warn!("bridge process died ({msg}); respawning and retrying");
                *slot = None;
                self.request_once(slot, op)
            }
            Err(e) => Err(e),
        }
    }

    fn request_once(
        &self,
        slot: &mut Option<ServeConnection>,
        op: &serde_json::Value,
    ) -> BridgeResult<Box<RawValue>> {
        if slot.is_none() {
            *slot = Some(self.spawn_serve()?);
        }
        let conn = slot.as_mut().expect("connection just ensured");

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

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        debug!(?cmd, "spawning bridge serve process");
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin just set");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout just set"));
        let stderr = child.stderr.take().expect("piped stderr just set");

        spawn_stderr_forwarder(stderr);

        Ok(ServeConnection {
            child: Some(child),
            stdin,
            stdout,
        })
    }
}

/// Serve processes for a batch of cold modules. Each one keeps a core busy and
/// peaks around 0.5 GB, so stay at a quarter of the cores and never above three.
fn parallel_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get() / 4)
        .clamp(1, 3)
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
pub(crate) fn forward_bridge_line(line: &str) {
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
