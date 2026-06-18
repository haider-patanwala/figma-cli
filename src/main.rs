//! figma-cli (Rust, Safe Mode) — control Figma Desktop via the FigCli plugin
//! bridge, no API key. This is the CLI shell + daemon; node creation runs as
//! JS inside Figma through the plugin (see daemon.rs).

mod config;
mod daemon;
mod engine;
mod lifecycle;
mod transport;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::Value;

const PLUGIN_MANIFEST: &str = include_str!("../assets/plugin/manifest.json");
const PLUGIN_CODE: &str = include_str!("../assets/plugin/code.js");
const PLUGIN_UI: &str = include_str!("../assets/plugin/ui.html");

#[derive(Parser)]
#[command(name = "figma-cli", version, about = "Control Figma Desktop via the FigCli plugin bridge (no API key).")]
struct Cli {
    /// Emit machine-readable JSON instead of human output.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon and connect to Figma (Safe Mode, via the FigCli plugin).
    Connect,
    /// Manage the background daemon.
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Run raw Figma Plugin API JavaScript and print the result.
    Eval {
        /// JS expression/statements; `figma` is the global Plugin API.
        code: String,
    },
    /// Render a single JSX frame into Figma.
    Render {
        /// JSX string, e.g. '<Frame bg="#fff" p={16}><Text>Hi</Text></Frame>'
        jsx: String,
        /// Pin var: lookups to a named variable collection.
        #[arg(long)]
        collection: Option<String>,
    },
    /// Render multiple JSX frames as independent top-level nodes.
    RenderBatch {
        /// JSON array of JSX strings.
        json_array: String,
        /// Lay out horizontally (default) or vertically.
        #[arg(long, default_value = "row")]
        direction: String,
        #[arg(long, default_value_t = 40.0)]
        gap: f64,
        #[arg(long)]
        collection: Option<String>,
    },
    /// Create a named design-token preset (e.g. shadcn).
    Tokens {
        #[command(subcommand)]
        action: TokensAction,
    },
    /// Find nodes whose name contains the query.
    Find {
        query: String,
    },
    /// Print info about the current page / canvas.
    Canvas {
        #[command(subcommand)]
        action: CanvasAction,
    },
    /// Screenshot a node (or selection) to a PNG for verification.
    Verify {
        /// Node id to capture; omit to use the current selection.
        node_id: Option<String>,
        /// Output PNG path (default: /tmp/figma-verify-<id>.png).
        #[arg(long)]
        save: Option<String>,
        /// Also include a measurement tree (real w/h, layout sizing).
        #[arg(long)]
        measure: bool,
    },
    /// Node operations on existing nodes.
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
    /// Lift a wrapper's children up to its parent, then delete the wrapper.
    Unwrap {
        node_id: String,
        /// Keep the (now-empty) wrapper instead of deleting it.
        #[arg(long)]
        keep_wrapper: bool,
    },
    /// Remove the node(s) created by the most recent render / render-batch.
    Undo,
    /// Variable / design-token operations.
    Var {
        #[command(subcommand)]
        action: VarAction,
    },
    /// Export a node (or selection) to a file (png/svg/jpg/pdf).
    Export {
        /// Format: png, svg, jpg, pdf.
        #[arg(default_value = "png")]
        format: String,
        /// Node id; omit to use the current selection.
        node_id: Option<String>,
        /// Output path (default: ./export-<id>.<ext>).
        #[arg(short, long)]
        output: Option<String>,
        /// Raster scale (png/jpg only).
        #[arg(long, default_value_t = 2.0)]
        scale: f64,
    },
    /// Hidden: run the daemon in the foreground (used internally).
    #[command(hide = true, name = "daemon-run")]
    DaemonRun,
}

#[derive(Subcommand)]
enum DaemonAction {
    Start,
    Status,
    Stop,
    Restart,
}

#[derive(Subcommand)]
enum TokensAction {
    /// Install a preset by name (currently: shadcn).
    Preset { name: String },
}

#[derive(Subcommand)]
enum CanvasAction {
    Info,
}

#[derive(Subcommand)]
enum NodeAction {
    /// Convert frame(s)/group(s) to components.
    ToComponent { node_ids: Vec<String> },
    /// Delete node(s) by id.
    Delete { node_ids: Vec<String> },
}

#[derive(Subcommand)]
enum VarAction {
    /// List variable collections and their variables.
    List,
    /// Delete all local variables and collections (optionally one collection).
    DeleteAll {
        #[arg(short, long)]
        collection: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("✗ {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let json = cli.json;
    match cli.command {
        Commands::DaemonRun => daemon::run().await,
        Commands::Connect => cmd_connect(json).await,
        Commands::Daemon { action } => cmd_daemon(action, json).await,
        Commands::Eval { code } => {
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        Commands::Render { jsx, collection } => {
            let req = transport::ExecRequest {
                action: "render".into(),
                jsx: Some(jsx),
                collection,
                ..Default::default()
            };
            lifecycle::ensure_started().await?;
            let v = transport::exec(req).await?;
            print_result(&v, json);
            Ok(())
        }
        Commands::RenderBatch { json_array, direction, gap, collection } => {
            let arr: Vec<String> = serde_json::from_str(&json_array)
                .map_err(|e| anyhow::anyhow!("json_array must be a JSON array of JSX strings: {e}"))?;
            let req = transport::ExecRequest {
                action: "render-batch".into(),
                jsx_array: Some(arr),
                gap: Some(gap),
                vertical: Some(direction == "col" || direction == "column" || direction == "vertical"),
                collection,
                ..Default::default()
            };
            lifecycle::ensure_started().await?;
            let v = transport::exec(req).await?;
            print_result(&v, json);
            Ok(())
        }
        Commands::Tokens { action } => match action {
            TokensAction::Preset { name } => {
                let code = engine::tokens_preset(&name)?;
                let v = exec_eval(&code).await?;
                print_result(&v, json);
                Ok(())
            }
        },
        Commands::Find { query } => {
            let code = format!(
                "return figma.currentPage.findAll(n => n.name.toLowerCase().includes({})).map(n => ({{ id: n.id, name: n.name, type: n.type }}))",
                js_string(&query.to_lowercase())
            );
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        Commands::Canvas { action } => match action {
            CanvasAction::Info => {
                let code = "return { page: figma.currentPage.name, selection: figma.currentPage.selection.length, children: figma.currentPage.children.length }".to_string();
                let v = exec_eval(&code).await?;
                print_result(&v, json);
                Ok(())
            }
        },
        Commands::Verify { node_id, save, measure } => cmd_verify(node_id, save, measure, json).await,
    }
}

async fn cmd_verify(node_id: Option<String>, save: Option<String>, measure: bool, json: bool) -> Result<()> {
    let node_lookup = match &node_id {
        Some(id) => format!("node = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const sel = figma.currentPage.selection; node = sel.length > 0 ? sel[0] : null;".to_string(),
    };
    // Mirrors src/commands/export-eval.js `verify`: scale-fit under 2000px, PNG, base64.
    let code = format!(
        r#"(async () => {{
  let node;
  {node_lookup}
  if (!node) return {{ error: 'No node selected or found' }};
  if (!('exportAsync' in node)) return {{ error: 'Node cannot be exported' }};
  const w = node.width || 100, h = node.height || 100;
  let scale = 2; const maxDim = 2000; const maxNodeDim = Math.max(w, h);
  if (maxNodeDim * scale > maxDim) scale = maxDim / maxNodeDim;
  const bytes = await node.exportAsync({{ format: 'PNG', constraint: {{ type: 'SCALE', value: scale }} }});
  const base64 = figma.base64Encode(bytes);
  let measure = null;
  if ({measure}) {{
    const walk = (n, depth) => {{
      const m = {{ name: n.name, type: n.type, w: Math.round(n.width), h: Math.round(n.height),
        layout: n.layoutMode && n.layoutMode !== 'NONE' ? n.layoutMode : undefined,
        sizeH: n.layoutSizingHorizontal, sizeV: n.layoutSizingVertical }};
      if (depth > 0 && 'children' in n && n.children.length) m.children = n.children.slice(0, 24).map(c => walk(c, depth - 1));
      return m;
    }};
    measure = walk(node, 3);
  }}
  return {{ name: node.name, id: node.id, width: Math.round(w*scale), height: Math.round(h*scale), scale, base64, measure }};
}})()"#
    );

    let v = exec_eval(&code).await?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!(err.to_string());
    }
    let b64 = v.get("base64").and_then(|b| b.as_str()).unwrap_or("");
    let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("node").replace(':', "-");
    let path = save.unwrap_or_else(|| format!("/tmp/figma-verify-{id}.png"));

    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| anyhow::anyhow!("decode screenshot: {e}"))?;
    std::fs::write(&path, &bytes)?;

    // Keep context lean: emit dims + path, never the base64 blob.
    let out = serde_json::json!({
        "name": v.get("name"),
        "id": v.get("id"),
        "width": v.get("width"),
        "height": v.get("height"),
        "saved": path,
        "measure": v.get("measure"),
    });
    if json {
        print_result(&out, true);
    } else {
        println!("✓ saved {} ({}x{})", path, v.get("width").and_then(|x| x.as_i64()).unwrap_or(0), v.get("height").and_then(|x| x.as_i64()).unwrap_or(0));
        if measure {
            println!("{}", serde_json::to_string_pretty(v.get("measure").unwrap_or(&serde_json::Value::Null)).unwrap_or_default());
        }
    }
    Ok(())
}

/// Ensure the daemon is up, then eval code via the plugin.
async fn exec_eval(code: &str) -> Result<Value> {
    lifecycle::ensure_started().await?;
    let req = transport::ExecRequest {
        action: "eval".into(),
        code: Some(code.to_string()),
        ..Default::default()
    };
    transport::exec(req).await
}

async fn cmd_connect(json: bool) -> Result<()> {
    let started = lifecycle::ensure_started().await?;
    let plugin_dir = config::config_dir().join("plugin");
    std::fs::create_dir_all(&plugin_dir)?;
    std::fs::write(plugin_dir.join("manifest.json"), PLUGIN_MANIFEST)?;
    std::fs::write(plugin_dir.join("code.js"), PLUGIN_CODE)?;
    std::fs::write(plugin_dir.join("ui.html"), PLUGIN_UI)?;

    // Poll for the plugin to connect.
    let mut connected = false;
    for _ in 0..30 {
        if let Ok(h) = transport::health().await {
            if h.get("plugin").and_then(|v| v.as_bool()).unwrap_or(false) {
                connected = true;
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    if json {
        print_result(
            &serde_json::json!({
                "daemonStarted": started,
                "pluginConnected": connected,
                "pluginDir": plugin_dir.to_string_lossy(),
            }),
            true,
        );
        return Ok(());
    }

    if connected {
        println!("✓ Connected to Figma (Safe Mode). Ready — what would you like to create?");
    } else {
        println!("Daemon running. Now connect the bridge plugin inside Figma:");
        println!();
        println!("  1. Open a design file in Figma Desktop");
        println!("  2. Menu → Plugins → Development → Import plugin from manifest…");
        println!("  3. Select: {}", plugin_dir.join("manifest.json").display());
        println!("  4. Run Plugins → Development → FigCli (keep its little window open)");
        println!();
        println!("Then re-run `figma-cli connect` (or any command) — it connects automatically.");
    }
    Ok(())
}

async fn cmd_daemon(action: DaemonAction, json: bool) -> Result<()> {
    match action {
        DaemonAction::Start => {
            let started = lifecycle::ensure_started().await?;
            print_status_line(if started { "daemon started" } else { "daemon already running" }, json);
            Ok(())
        }
        DaemonAction::Stop => {
            lifecycle::stop()?;
            print_status_line("daemon stopped", json);
            Ok(())
        }
        DaemonAction::Restart => {
            lifecycle::stop()?;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            lifecycle::ensure_started().await?;
            print_status_line("daemon restarted", json);
            Ok(())
        }
        DaemonAction::Status => {
            match transport::health().await {
                Ok(h) => print_result(&h, json),
                Err(_) => print_result(&serde_json::json!({ "status": "stopped" }), json),
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Output + helpers
// ---------------------------------------------------------------------------

fn print_result(v: &Value, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    } else {
        match v {
            Value::String(s) => println!("{s}"),
            Value::Null => println!("(ok)"),
            other => println!("{}", serde_json::to_string_pretty(other).unwrap_or_default()),
        }
    }
}

fn print_status_line(msg: &str, json: bool) {
    if json {
        println!("{}", serde_json::json!({ "status": msg }));
    } else {
        println!("✓ {msg}");
    }
}

/// JSON-encode a string for safe interpolation into generated JS.
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}
