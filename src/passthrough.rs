//! Wrapper for commands not yet reimplemented in Rust (DESIGN.md extract/import/
//! spec/instantiate, code-import, blocks, url-tools). These carry heavy Node-side
//! logic (YAML, headless browser, large markdown round-trips); per the project
//! plan they fall back to the original Node CLI, which speaks this daemon's exact
//! HTTP protocol (port 3456 + x-daemon-token), so it drives the *same* running
//! Rust daemon and the same FigCli plugin.
//!
//! The Node CLI is located via (in order): $FIGMA_CLI_JS, a `js/index.js` next to
//! the binary (skill layout), or `skill/js/index.js` within the crate. If none is
//! found (or Node isn't installed), we print a clear, actionable message instead
//! of failing opaquely.

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;

fn find_js_cli() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FIGMA_CLI_JS") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let mut candidates = vec![];
    if let Some(dir) = exe.parent() {
        candidates.push(dir.join("js/index.js"));
        candidates.push(dir.join("../js/index.js"));
        // dev layout: rust/target/{debug,release}/figma-cli -> rust/skill/js/index.js
        // (self-contained inside the crate; no dependency on the repo root)
        candidates.push(dir.join("../../skill/js/index.js"));
    }
    candidates.into_iter().find(|p| p.exists())
}

fn node_available() -> bool {
    Command::new("node").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Forward `args` (the full subcommand + flags) to the Node CLI, inheriting
/// stdio so its output reaches the user directly. Returns the child exit code.
pub fn run(args: &[String]) -> Result<i32> {
    let cli = find_js_cli().ok_or_else(|| anyhow!(
        "this command is not yet ported to the Rust binary and the Node fallback was not found.\n\
         Set FIGMA_CLI_JS=/path/to/figma-cli/src/index.js (or place the JS CLI at `js/index.js` next to the binary)."
    ))?;
    if !node_available() {
        return Err(anyhow!(
            "this command falls back to the Node implementation, but `node` is not on PATH. Install Node 18+ and retry."
        ));
    }
    let status = Command::new("node")
        .arg(&cli)
        .args(args)
        .status()
        .map_err(|e| anyhow!("failed to run node fallback: {e}"))?;
    Ok(status.code().unwrap_or(1))
}
