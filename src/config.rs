//! Shared paths, port, and daemon auth token.
//!
//! Mirrors the conventions in the original JS CLI (`src/lib/cli-core.js`):
//! daemon listens on 127.0.0.1:3456, pidfile in $HOME, token in
//! ~/.figma-ds-cli/.daemon-token. The token guards the HTTP `/exec` and
//! `/health` routes; the plugin WebSocket at `/plugin` is unauthenticated
//! (it only ever connects from inside Figma on loopback).

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub const DAEMON_PORT: u16 = 3456;
pub const DAEMON_HOST: &str = "127.0.0.1";

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn pid_file() -> PathBuf {
    home().join(".figma-cli-daemon.pid")
}

pub fn config_dir() -> PathBuf {
    home().join(".figma-ds-cli")
}

pub fn token_file() -> PathBuf {
    config_dir().join(".daemon-token")
}

/// Read the daemon token, or create one if it does not exist yet.
pub fn ensure_token() -> Result<String> {
    let path = token_file();
    if let Ok(tok) = fs::read_to_string(&path) {
        let tok = tok.trim().to_string();
        if !tok.is_empty() {
            return Ok(tok);
        }
    }
    let token = random_token();
    fs::create_dir_all(config_dir()).context("create config dir")?;
    fs::write(&path, &token).context("write token file")?;
    // Best-effort tighten perms (0600) on unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(token)
}

/// Read the token without creating it (used by the CLI client).
pub fn read_token() -> Result<String> {
    let path = token_file();
    let tok = fs::read_to_string(&path)
        .with_context(|| format!("daemon token not found at {} (is the daemon running? try `figma-cli connect`)", path.display()))?;
    Ok(tok.trim().to_string())
}

fn random_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..48)
        .map(|_| {
            let n: u8 = rng.gen_range(0..16);
            std::char::from_digit(n as u32, 16).unwrap()
        })
        .collect()
}

pub fn base_url() -> String {
    format!("http://{}:{}", DAEMON_HOST, DAEMON_PORT)
}
