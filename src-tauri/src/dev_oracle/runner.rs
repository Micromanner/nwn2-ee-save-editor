use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;

use crate::services::toolset_bridge::forward_bridge_line;

const TIMEOUT: Duration = Duration::from_secs(120);
const BUILD_HINT: &str = "dotnet build toolset-oracle -c Release -p:Platform=x64";

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<String>,
}

fn oracle_exe() -> PathBuf {
    std::env::var_os("NWN2_ORACLE_EXE").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../toolset-oracle/bin/x64/Release/net48/toolset-oracle.exe")
        },
        PathBuf::from,
    )
}

/// Runs one oracle op in a fresh `toolset-oracle` process. Blocking; call it from
/// `spawn_blocking`.
pub fn run(install: &Path, op: &str, request: &Value) -> Result<Value, String> {
    let exe = oracle_exe();
    if !exe.is_file() {
        return Err(format!(
            "toolset-oracle not built ({}); run `{BUILD_HINT}` in the repo root",
            exe.display()
        ));
    }

    let mut cmd = Command::new(&exe);
    cmd.arg("--nwn2-install")
        .arg(install)
        .arg(op)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start oracle: {e}"))?;
    let stdout = drain(child.stdout.take().expect("piped stdout"));
    let stderr = drain(child.stderr.take().expect("piped stderr"));
    let mut stdin = child.stdin.take().expect("piped stdin");
    let body = request.to_string();
    let send = std::thread::spawn(move || stdin.write_all(body.as_bytes()));

    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                return Err(format!("oracle op '{op}' timed out after {TIMEOUT:?}"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("failed to wait for oracle: {e}")),
        }
    }

    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();
    for line in err.lines().filter(|l| !l.is_empty()) {
        forward_bridge_line(line);
    }

    let sent = match send.join() {
        Ok(Ok(())) => String::new(),
        Ok(Err(e)) => format!("; sending the request failed: {e}"),
        Err(_) => "; sending the request panicked".to_string(),
    };
    let envelope: Envelope = serde_json::from_str(out.trim()).map_err(|e| {
        format!(
            "oracle returned no valid response ({e}){sent}; stderr:\n{}",
            tail(&err)
        )
    })?;
    if envelope.ok {
        envelope
            .data
            .ok_or_else(|| "oracle response has no data".to_string())
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "unknown oracle error".to_string()))
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(20)..].join("\n")
}
