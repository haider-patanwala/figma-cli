//! CLI -> daemon HTTP client.
//!
//! The CLI is short-lived: it builds a request, posts it to the long-lived
//! daemon's `/exec` endpoint, and prints the result. Request body matches the
//! JS daemon (`src/daemon.js` handleRequest): `{action, code|jsx|jsxArray, ...}`.

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;

use crate::config;

#[derive(Serialize, Default)]
pub struct ExecRequest {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx: Option<String>,
    #[serde(rename = "jsxArray", skip_serializing_if = "Option::is_none")]
    pub jsx_array: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
}

/// POST to the daemon `/exec` endpoint and return the `result` value.
pub async fn exec(req: ExecRequest) -> Result<Value> {
    let token = config::read_token()?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/exec", config::base_url()))
        .header("x-daemon-token", token)
        .json(&req)
        .send()
        .await
        .map_err(|e| anyhow!("could not reach daemon at {} ({e}). Run `figma-cli connect` first.", config::base_url()))?;

    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| anyhow!("bad daemon response: {e}"))?;

    if !status.is_success() {
        let msg = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown daemon error");
        return Err(anyhow!(msg.to_string()));
    }
    Ok(body.get("result").cloned().unwrap_or(Value::Null))
}

/// GET /health. Returns the parsed JSON, or Err if the daemon is unreachable.
pub async fn health() -> Result<Value> {
    let token = config::read_token().unwrap_or_default();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/health", config::base_url()))
        .header("x-daemon-token", token)
        .timeout(std::time::Duration::from_millis(2000))
        .send()
        .await?;
    Ok(resp.json().await?)
}
