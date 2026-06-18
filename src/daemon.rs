//! The long-lived daemon: HTTP server (CLI <-> daemon) + WebSocket server
//! (daemon <-> Figma plugin), sharing one listener on 127.0.0.1:3456.
//!
//! Protocol parity with the JS daemon (`src/daemon.js`) so the *unmodified*
//! FigCli plugin (`plugin/ui.html`, `plugin/code.js`) connects with no changes:
//!   - daemon -> plugin: {action:"eval",id,code} | {action:"eval-batch",id,codes} | {type:"pong"}
//!   - plugin -> daemon: {type:"hello"} | {type:"ping"} | {type:"result",id,result,error}
//!                       | {type:"batch-result",id,results}
//! HTTP routes mirror the JS daemon: GET /health, POST /exec (token-guarded).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::config;
use crate::engine::{self, RenderOpts};

const EVAL_TIMEOUT: Duration = Duration::from_secs(25);
const BATCH_TIMEOUT: Duration = Duration::from_secs(60);

type Pending = oneshot::Sender<std::result::Result<Value, String>>;

struct AppState {
    token: String,
    next_id: AtomicU64,
    /// Sender to the currently-connected plugin's outbound queue (None if no plugin).
    plugin_tx: Mutex<Option<mpsc::UnboundedSender<Message>>>,
    pending: Mutex<HashMap<u64, Pending>>,
}

impl AppState {
    fn plugin_connected(&self) -> bool {
        // try_lock keeps /health cheap; treat contention as "connected".
        self.plugin_tx
            .try_lock()
            .map(|g| g.is_some())
            .unwrap_or(true)
    }
}

/// Run the daemon in the foreground (invoked via the hidden `daemon-run` subcommand).
pub async fn run() -> Result<()> {
    let token = config::ensure_token()?;
    let state = Arc::new(AppState {
        token,
        next_id: AtomicU64::new(0),
        plugin_tx: Mutex::new(None),
        pending: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/exec", post(exec))
        .route("/plugin", get(plugin_ws))
        .with_state(state);

    let addr = format!("{}:{}", config::DAEMON_HOST, config::DAEMON_PORT);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[daemon] figma-cli daemon running on {addr} (Safe Mode)");
    eprintln!("[daemon] waiting for FigCli plugin connection at ws://{addr}/plugin");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("[daemon] shutting down");
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

fn check_auth(state: &AppState, headers: &HeaderMap) -> Result<(), String> {
    // Layer 1: Host header (block DNS rebinding) — matches JS daemon.
    if let Some(host) = headers.get("host").and_then(|h| h.to_str().ok()) {
        let ok = host.starts_with("localhost") || host.starts_with("127.0.0.1");
        if !ok {
            return Err("Invalid host header".into());
        }
    }
    // Layer 2: session token.
    let token = headers
        .get("x-daemon-token")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if token != state.token {
        return Err("Invalid or missing token".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP routes
// ---------------------------------------------------------------------------

async fn health(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = check_auth(&state, &headers) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": format!("Unauthorized: {e}") })));
    }
    let connected = state.plugin_connected();
    (
        StatusCode::OK,
        Json(json!({
            "status": if connected { "ok" } else { "disconnected" },
            "mode": "safe",
            "plugin": connected,
        })),
    )
}

async fn exec(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    if let Err(e) = check_auth(&state, &headers) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": format!("Unauthorized: {e}") })));
    }

    let action = payload.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let result = match action {
        "eval" => {
            let code = payload.get("code").and_then(|v| v.as_str()).unwrap_or("");
            eval(&state, code).await
        }
        "render" => {
            let jsx = payload.get("jsx").and_then(|v| v.as_str()).unwrap_or("");
            let collection = payload.get("collection").and_then(|v| v.as_str());
            match engine::parse_jsx(jsx, collection) {
                Ok(code) => eval(&state, &code).await,
                Err(e) => Err(e.to_string()),
            }
        }
        "render-batch" => {
            let arr: Vec<String> = payload
                .get("jsxArray")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let opts = RenderOpts {
                gap: payload.get("gap").and_then(|v| v.as_f64()).unwrap_or(40.0),
                vertical: payload.get("vertical").and_then(|v| v.as_bool()).unwrap_or(false),
                collection: payload.get("collection").and_then(|v| v.as_str()).map(String::from),
            };
            match engine::parse_jsx_batch(&arr, &opts) {
                Ok(code) => eval_batch(&state, &[code]).await,
                Err(e) => Err(e.to_string()),
            }
        }
        other => Err(format!("Unknown action: {other}")),
    };

    match result {
        Ok(v) => (StatusCode::OK, Json(json!({ "result": v, "mode": "safe" }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))),
    }
}

// ---------------------------------------------------------------------------
// Eval dispatch (daemon -> plugin)
// ---------------------------------------------------------------------------

async fn eval(state: &AppState, code: &str) -> std::result::Result<Value, String> {
    dispatch(state, "eval", json!(code), EVAL_TIMEOUT).await
}

async fn eval_batch(state: &AppState, codes: &[String]) -> std::result::Result<Value, String> {
    dispatch(state, "eval-batch", json!(codes), BATCH_TIMEOUT).await
}

/// Send an action to the plugin and await its keyed reply.
async fn dispatch(
    state: &AppState,
    action: &str,
    payload: Value,
    timeout: Duration,
) -> std::result::Result<Value, String> {
    let tx = {
        let guard = state.plugin_tx.lock().await;
        guard.clone()
    };
    let Some(tx) = tx else {
        return Err("Plugin not connected. Start the FigCli plugin in Figma.".into());
    };

    let id = state.next_id.fetch_add(1, Ordering::SeqCst) + 1;
    let (resp_tx, resp_rx) = oneshot::channel();
    state.pending.lock().await.insert(id, resp_tx);

    let msg = if action == "eval-batch" {
        json!({ "action": "eval-batch", "id": id, "codes": payload })
    } else {
        json!({ "action": "eval", "id": id, "code": payload })
    };

    if tx.send(Message::Text(msg.to_string())).is_err() {
        state.pending.lock().await.remove(&id);
        return Err("Plugin connection closed before send".into());
    }

    match tokio::time::timeout(timeout, resp_rx).await {
        Ok(Ok(res)) => res,
        Ok(Err(_)) => Err("Plugin disconnected".into()),
        Err(_) => {
            state.pending.lock().await.remove(&id);
            Err(format!("Plugin execution timeout ({}s)", timeout.as_secs()))
        }
    }
}

// ---------------------------------------------------------------------------
// WebSocket: plugin connection
// ---------------------------------------------------------------------------

async fn plugin_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_plugin(socket, state))
}

async fn handle_plugin(socket: WebSocket, state: Arc<AppState>) {
    eprintln!("[daemon] plugin connected (Safe Mode)");
    let (mut sink, mut stream) = socket.split();

    // Outbound queue: anything pushed here is forwarded to the plugin socket.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    {
        let mut guard = state.plugin_tx.lock().await;
        *guard = Some(out_tx.clone());
    }

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Inbound loop: handle plugin messages.
    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(text) = msg {
            handle_plugin_message(&state, &out_tx, &text).await;
        } else if let Message::Close(_) = msg {
            break;
        }
    }

    // Plugin gone: clear sender, reject all pending requests.
    {
        let mut guard = state.plugin_tx.lock().await;
        *guard = None;
    }
    {
        let mut pending = state.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err("Plugin disconnected".into()));
        }
    }
    writer.abort();
    eprintln!("[daemon] plugin disconnected");
}

async fn handle_plugin_message(
    state: &AppState,
    out_tx: &mpsc::UnboundedSender<Message>,
    text: &str,
) {
    let Ok(msg) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let mtype = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match mtype {
        "hello" => {
            let v = msg.get("version").and_then(|v| v.as_str()).unwrap_or("?");
            eprintln!("[daemon] plugin version: {v}");
        }
        "ping" => {
            let _ = out_tx.send(Message::Text(json!({ "type": "pong" }).to_string()));
        }
        "pong" => {}
        "result" => {
            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                if let Some(tx) = state.pending.lock().await.remove(&id) {
                    if let Some(err) = msg.get("error").and_then(|v| v.as_str()) {
                        let _ = tx.send(Err(err.to_string()));
                    } else {
                        let _ = tx.send(Ok(msg.get("result").cloned().unwrap_or(Value::Null)));
                    }
                }
            }
        }
        "batch-result" => {
            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                if let Some(tx) = state.pending.lock().await.remove(&id) {
                    let _ = tx.send(Ok(msg.get("results").cloned().unwrap_or(Value::Null)));
                }
            }
        }
        _ => {}
    }
}
