//! Daemon process lifecycle: spawn detached, status, stop.
//!
//! The same binary is both CLI and daemon. `daemon start` spawns the binary
//! with the hidden `daemon-run` subcommand detached, recording its PID in
//! ~/.figma-cli-daemon.pid (mirrors `src/lib/cli-core.js`).

use anyhow::{anyhow, Result};
use std::fs;
use std::process::Command;
use std::time::Duration;

use crate::{config, transport};

/// Is the daemon answering on its port?
pub async fn is_running() -> bool {
    transport::health().await.is_ok()
}

/// Spawn the daemon detached if it isn't already running. Returns true if a new
/// process was started.
pub async fn ensure_started() -> Result<bool> {
    if is_running().await {
        return Ok(false);
    }
    let exe = std::env::current_exe()?;
    // Ensure a token exists before the daemon (and CLI) need it.
    config::ensure_token()?;

    let child = Command::new(exe)
        .arg("daemon-run")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to spawn daemon: {e}"))?;

    fs::write(config::pid_file(), child.id().to_string())?;

    // Wait for the HTTP server to come up (up to ~5s).
    for _ in 0..50 {
        if is_running().await {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow!("daemon did not become healthy within 5s"))
}

pub fn read_pid() -> Option<u32> {
    fs::read_to_string(config::pid_file())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Stop the daemon (SIGTERM on unix) and remove the pidfile.
pub fn stop() -> Result<()> {
    if let Some(pid) = read_pid() {
        #[cfg(unix)]
        {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
        #[cfg(not(unix))]
        {
            let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).status();
        }
    }
    let _ = fs::remove_file(config::pid_file());
    Ok(())
}
