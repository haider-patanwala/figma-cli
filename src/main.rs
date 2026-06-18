//! figma-cli (Rust, Safe Mode) — control Figma Desktop via the FigCli plugin
//! bridge, no API key. This is the CLI shell + daemon; node creation runs as
//! JS inside Figma through the plugin (see daemon.rs).

mod cmds;
mod config;
mod daemon;
mod engine;
mod jsgen;
mod lifecycle;
mod passthrough;
mod tools;
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
    /// Create elements (frame, rect, ellipse, text, line, component, group, autolayout).
    Create {
        #[command(subcommand)]
        action: CreateAction,
    },
    /// Set properties on the selection / a node / nodes matching a query.
    Set {
        #[command(subcommand)]
        action: SetAction,
    },
    /// Control auto-layout sizing (hug / fill / fixed).
    Sizing {
        #[command(subcommand)]
        action: SizingAction,
    },
    /// Bind a variable to a node property (fill/stroke/radius/gap/padding).
    Bind {
        #[command(subcommand)]
        action: BindAction,
    },
    /// Select a node by id.
    Select { node_id: String },
    /// Delete a node by id, or the current selection.
    Delete { node_id: Option<String> },
    /// Duplicate a node by id, or the current selection.
    Duplicate {
        node_id: Option<String>,
        #[arg(long, default_value_t = 20.0)]
        offset: f64,
    },
    /// Print details of a node by id, or the current selection.
    Get { node_id: Option<String> },
    /// Set padding (CSS-style 1-4 values) on the selection.
    Padding { value: f64, r: Option<f64>, b: Option<f64>, l: Option<f64> },
    /// Set auto-layout gap on the selection.
    Gap { value: f64 },
    /// Align items: start, center, end, stretch.
    Align { alignment: String },
    /// Accessibility checks.
    A11y {
        #[command(subcommand)]
        action: A11yAction,
    },
    /// Inspect nodes (tree, bindings).
    NodeTree {
        node_id: Option<String>,
        #[arg(short, long, default_value_t = 3)]
        depth: u32,
    },
    /// Show a node's variable bindings.
    NodeBindings { node_id: Option<String> },
    /// Lint the design for issues (naming, hardcoded colors, tiny text).
    Lint,
    /// Analyze the design (colors, typography, spacing, clusters).
    Analyze {
        #[command(subcommand)]
        action: AnalyzeAction,
    },
    /// Slot operations (create/list/reset/convert).
    Slot {
        #[command(subcommand)]
        action: SlotAction,
    },
    /// Combine frames/components into a variant set.
    Variants {
        #[command(subcommand)]
        action: VariantsAction,
    },
    /// Component property operations (combine into a variant set).
    Prop {
        #[command(subcommand)]
        action: PropAction,
    },
    /// Export design tokens as code (css / tailwind / dtcg).
    ExportTokens {
        #[command(subcommand)]
        action: ExportTokensAction,
    },
    /// Get/set local CLI config (e.g. removebgApiKey).
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// FigJam operations (sticky/shape/text/connect/move/update/info/nodes).
    Figjam {
        #[command(subcommand)]
        action: FigjamAction,
    },
    /// Delete multiple nodes (comma-separated ids or JSON array).
    DeleteBatch { node_ids: String },
    /// Extract / generate gradients.
    Gradient {
        #[command(subcommand)]
        action: GradientAction,
    },
    /// shadcn/ui components (requires `tokens preset shadcn`).
    Shadcn {
        #[command(subcommand)]
        action: ShadcnAction,
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
    /// Export the open file as a DESIGN.md (Node fallback). [output] [--pages …] [--sections …] [--selection]
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Extract { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Import tokens/DESIGN.md/tailwind.config/css/storybook into variables (Node fallback).
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Import { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Show a component's spec from a DESIGN.md (Node fallback). <component> [--check <id>]
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Spec { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Instance a component from an extracted system (Node fallback). <name>
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Instantiate { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Pre-built UI blocks (Node fallback). `blocks list` | `blocks create <id>`
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Blocks { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Recreate a webpage in Figma (Node fallback). <url>
    #[command(name = "recreate-url", trailing_var_arg = true, disable_help_flag = true)]
    RecreateUrl { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Screenshot a webpage into Figma (Node fallback). <url>
    #[command(name = "screenshot-url", trailing_var_arg = true, disable_help_flag = true)]
    ScreenshotUrl { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Remove a node's background via remove.bg (Node fallback). [nodeId]
    #[command(name = "remove-bg", trailing_var_arg = true, disable_help_flag = true)]
    RemoveBg { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Dev resource links on nodes (Node fallback). link|list|unlink|edit …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Dev { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Sections (Node fallback). create|list|add …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Section { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Layout grids (Node fallback). set|list|clear …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Grid { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Annotations on nodes (Node fallback). add|list|clear …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Annotate { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Bundled plugins (Node fallback). list|install|setup …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Plugins { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Figma Plugin API docs (Node fallback). [name]
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Api { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Generate size variants (Node fallback). [nodeId] …
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Sizes { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Show all variant combinations (Node fallback). [nodeId]
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Combos { #[arg(allow_hyphen_values = true)] args: Vec<String> },
    /// Escape hatch: forward any args to the original Node CLI (advanced).
    #[command(trailing_var_arg = true, disable_help_flag = true)]
    Js { #[arg(allow_hyphen_values = true)] args: Vec<String> },
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
    /// Create a 4px-base spacing scale.
    Spacing { #[arg(short, long, default_value = "Spacing")] collection: String },
    /// Create a border-radius scale.
    Radii { #[arg(short, long, default_value = "Radii")] collection: String },
}

#[derive(Subcommand)]
enum ExportTokensAction {
    /// Export variables as CSS custom properties.
    Css,
    /// Export color variables as a Tailwind config.
    Tailwind,
    /// Export variables as W3C Design Tokens (DTCG) JSON.
    Dtcg { output: Option<String> },
}

#[derive(Subcommand)]
enum ConfigAction {
    Set { key: String, value: String },
    Get { key: String },
}

#[derive(Subcommand)]
enum ShadcnAction {
    /// List available shadcn components.
    List,
    /// Add component(s) to the canvas. Names, or --all; --count for copies.
    Add {
        names: Vec<String>,
        #[arg(long)] all: bool,
        #[arg(short, long, default_value_t = 1)] count: u32,
    },
}

#[derive(Subcommand)]
enum GradientAction {
    /// Extract a gradient from an image. --mode linear|mesh, --apply-to <id>.
    Extract {
        image: String,
        #[arg(long, default_value = "linear")] mode: String,
        #[arg(long)] apply_to: Option<String>,
        #[arg(long, default_value = "auto")] direction: String,
        #[arg(long, default_value_t = 3)] stops: u32,
        #[arg(long)] blur: Option<f64>,
        #[arg(long)] no_trim: bool,
    },
    /// Generate a mesh-gradient wallpaper from a palette. e.g. "#a,#b,#c".
    Mesh {
        colors: String,
        #[arg(long)] apply_to: Option<String>,
        #[arg(long)] base: Option<String>,
        #[arg(long, default_value = "1920x1080")] size: String,
        #[arg(long)] blur: Option<f64>,
        #[arg(long, default_value = "auto")] style: String,
        #[arg(long)] seed: Option<i64>,
        #[arg(long, default_value = "Mesh Wallpaper")] name: String,
    },
}

#[derive(Subcommand)]
enum FigjamAction {
    Sticky { text: String, #[arg(short, long, default_value_t = 0.0)] x: f64, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(short, long)] color: Option<String> },
    Shape { text: String, #[arg(short, long, default_value_t = 0.0)] x: f64, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(short, long, default_value_t = 200.0)] width: f64, #[arg(short = 'H', long, default_value_t = 100.0)] height: f64, #[arg(short = 't', long, default_value = "ROUNDED_RECTANGLE")] r#type: String },
    Text { content: String, #[arg(short, long, default_value_t = 0.0)] x: f64, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(short, long, default_value_t = 16.0)] size: f64 },
    Connect { start_id: String, end_id: String },
    Delete { node_id: String },
    Move { node_id: String, x: f64, y: f64 },
    Update { node_id: String, text: String },
    Info,
    Nodes { #[arg(short, long, default_value_t = 20)] limit: u32 },
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
enum SlotAction {
    Create { name: String, #[arg(short, long, default_value = "col")] flex: String, #[arg(short, long, default_value_t = 0.0)] gap: f64, #[arg(short, long, default_value_t = 0.0)] padding: f64 },
    List { node_id: Option<String> },
    Reset { node_id: Option<String> },
    Convert { node_id: Option<String>, #[arg(short, long, default_value = "Slot")] name: String },
}

#[derive(Subcommand)]
enum VariantsAction {
    /// Combine comma-separated node ids into a variant set.
    From {
        ids: String,
        #[arg(short, long)] property: String,
        #[arg(short, long)] values: String,
        #[arg(short, long, default_value = "")] name: String,
    },
}

#[derive(Subcommand)]
enum PropAction {
    Combine { ids: String, #[arg(short, long, default_value = "ComponentSet")] name: String },
}

#[derive(Subcommand)]
enum AnalyzeAction {
    Colors,
    #[command(alias = "type")]
    Typography,
    Spacing,
    Clusters,
}

#[derive(Subcommand)]
enum A11yAction {
    Contrast { node_id: Option<String>, #[arg(long, default_value = "AA")] level: String },
    Touch { node_id: Option<String>, #[arg(long, default_value_t = 44)] min: u32 },
    Text { node_id: Option<String> },
    Audit { node_id: Option<String> },
}

#[derive(Subcommand)]
enum CreateAction {
    Frame { name: String, #[arg(short, long, default_value_t = 100.0)] width: f64, #[arg(short = 'H', long, default_value_t = 100.0)] height: f64, #[arg(short, long)] x: Option<f64>, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(long)] fill: Option<String>, #[arg(long)] radius: Option<f64> },
    #[command(alias = "rectangle")]
    Rect { name: Option<String>, #[arg(short, long, default_value_t = 100.0)] width: f64, #[arg(short = 'H', long, default_value_t = 100.0)] height: f64, #[arg(short, long)] x: Option<f64>, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(long)] fill: Option<String>, #[arg(long)] stroke: Option<String>, #[arg(long)] radius: Option<f64>, #[arg(long)] opacity: Option<f64> },
    #[command(alias = "circle")]
    Ellipse { name: Option<String>, #[arg(short, long, default_value_t = 100.0)] width: f64, #[arg(short = 'H', long)] height: Option<f64>, #[arg(short, long)] x: Option<f64>, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(long)] fill: Option<String>, #[arg(long)] stroke: Option<String> },
    Text { content: String, #[arg(short, long)] x: Option<f64>, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(short, long, default_value_t = 16.0)] size: f64, #[arg(short, long, default_value = "#000000")] color: String, #[arg(short, long, default_value = "regular")] weight: String, #[arg(long, default_value = "Inter")] font: String, #[arg(long)] width: Option<f64>, #[arg(long, default_value_t = 100.0)] spacing: f64 },
    Component { name: Option<String> },
    Group { name: Option<String> },
    #[command(alias = "al")]
    Autolayout { name: Option<String>, #[arg(short, long, default_value = "row")] direction: String, #[arg(short, long, default_value_t = 8.0)] gap: f64, #[arg(short, long, default_value_t = 16.0)] padding: f64, #[arg(short, long)] x: Option<f64>, #[arg(short, long, default_value_t = 0.0)] y: f64, #[arg(long)] fill: Option<String>, #[arg(long)] radius: Option<f64>, #[arg(long, default_value_t = 100.0)] spacing: f64 },
}

#[derive(Subcommand)]
enum SetAction {
    Fill { color: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Stroke { color: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Radius { value: f64, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Size { width: f64, height: f64, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Scale { factor: f64, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String>, #[arg(long)] keep_spacing: bool },
    #[command(alias = "position")]
    Pos { x: f64, y: f64, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Opacity { value: f64, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Name { name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Text { text: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
}

#[derive(Subcommand)]
enum SizingAction {
    Hug { #[arg(short, long, default_value = "both")] axis: String },
    Fill { #[arg(short, long, default_value = "both")] axis: String },
    Fixed { width: f64, height: Option<f64> },
}

#[derive(Subcommand)]
enum BindAction {
    Fill { var_name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Stroke { var_name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Radius { var_name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Gap { var_name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String> },
    Padding { var_name: String, #[arg(short, long)] node: Option<String>, #[arg(short, long)] query: Option<String>, #[arg(short, long, default_value = "all")] side: String },
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
    /// Create a single variable.
    Create {
        name: String,
        #[arg(short, long)] collection: String,
        #[arg(short, long)] r#type: String,
        #[arg(short, long)] value: Option<String>,
    },
    /// Find variables whose name contains a pattern.
    Find { pattern: String },
    /// Create color swatches on the canvas for a collection.
    Visualize { collection: Option<String> },
    /// Create multiple variables from a JSON array.
    CreateBatch { data: String, #[arg(short, long)] collection: String },
    /// Bind variables to multiple nodes from a JSON array.
    BindBatch { data: String },
    /// Set properties on multiple nodes from a JSON array (hex or var: refs).
    SetBatch { data: String, #[arg(short, long)] collection: Option<String> },
    /// Rename multiple nodes from a JSON array or object.
    RenameBatch { data: String },
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
            save_last_render(&v);
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
            save_last_render(&v);
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
            TokensAction::Spacing { collection } => {
                let v = run_asset(TOKENS_SPACING, serde_json::json!({ "collection": collection })).await?;
                print_result(&v, json); Ok(())
            }
            TokensAction::Radii { collection } => {
                let v = run_asset(TOKENS_RADII, serde_json::json!({ "collection": collection })).await?;
                print_result(&v, json); Ok(())
            }
        },
        Commands::ExportTokens { action } => {
            match action {
                ExportTokensAction::Css => { let v = exec_eval(&wrap_iife(EXPORT_CSS)).await?; print_result(&v, json); }
                ExportTokensAction::Tailwind => { let v = exec_eval(&wrap_iife(EXPORT_TAILWIND)).await?; print_result(&v, json); }
                ExportTokensAction::Dtcg { output } => {
                    let v = exec_eval(&wrap_iife(EXPORT_DTCG)).await?;
                    if let (Some(path), Some(s)) = (output, v.as_str()) {
                        std::fs::write(&path, format!("{s}\n"))?;
                        if json { print_result(&serde_json::json!({ "saved": path }), true); } else { println!("✓ wrote DTCG tokens → {path}"); }
                    } else {
                        print_result(&v, json);
                    }
                }
            }
            Ok(())
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Set { key, value } => { cfg_set(&key, &value)?; if !json { println!("✓ config saved: {key}"); } }
                ConfigAction::Get { key } => {
                    let val = cfg_get(&key)?;
                    match val { Some(v) => println!("{v}"), None => if !json { println!("(not set)") } }
                }
            }
            Ok(())
        }
        Commands::Figjam { action } => {
            let code = match action {
                FigjamAction::Sticky { text, x, y, color } => cmds::figjam_sticky(&text, x, y, color.as_deref()),
                FigjamAction::Shape { text, x, y, width, height, r#type } => cmds::figjam_shape(&text, x, y, width, height, &r#type),
                FigjamAction::Text { content, x, y, size } => cmds::figjam_text(&content, x, y, size),
                FigjamAction::Connect { start_id, end_id } => cmds::figjam_connect(&start_id, &end_id),
                FigjamAction::Delete { node_id } => cmds::delete(Some(&node_id)),
                FigjamAction::Move { node_id, x, y } => cmds::figjam_move(&node_id, x, y),
                FigjamAction::Update { node_id, text } => cmds::figjam_update(&node_id, &text),
                FigjamAction::Info => cmds::figjam_info().to_string(),
                FigjamAction::Nodes { limit } => cmds::figjam_nodes(limit),
            };
            let v = exec_eval(&code).await?; print_result(&v, json); Ok(())
        }
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
        Commands::Node { action } => cmd_node(action, json).await,
        Commands::Unwrap { node_id, keep_wrapper } => cmd_unwrap(node_id, keep_wrapper, json).await,
        Commands::Undo => cmd_undo(json).await,
        Commands::Var { action } => cmd_var(action, json).await,
        Commands::Export { format, node_id, output, scale } => cmd_export(format, node_id, output, scale, json).await,
        Commands::Create { action } => cmd_create(action, json).await,
        Commands::Set { action } => cmd_set(action, json).await,
        Commands::Sizing { action } => {
            let code = match action {
                SizingAction::Hug { axis } => cmds::sizing_hug(&axis),
                SizingAction::Fill { axis } => cmds::sizing_fill(&axis),
                SizingAction::Fixed { width, height } => cmds::sizing_fixed(width, height.unwrap_or(width)),
            };
            let v = exec_eval(&code).await?; print_result(&v, json); Ok(())
        }
        Commands::Bind { action } => cmd_bind(action, json).await,
        Commands::Select { node_id } => { let v = exec_eval(&cmds::select(&node_id)).await?; print_result(&v, json); Ok(()) }
        Commands::Delete { node_id } => { let v = exec_eval(&cmds::delete(node_id.as_deref())).await?; print_result(&v, json); Ok(()) }
        Commands::Duplicate { node_id, offset } => { let v = exec_eval(&cmds::duplicate(node_id.as_deref(), offset)).await?; print_result(&v, json); Ok(()) }
        Commands::Get { node_id } => { let v = exec_eval(&cmds::get(node_id.as_deref())).await?; print_result(&v, json); Ok(()) }
        Commands::Padding { value, r, b, l } => {
            // CSS-style 1-4 values.
            let (t, ri, bo, le) = match (r, b, l) {
                (None, _, _) => (value, value, value, value),
                (Some(r), None, _) => (value, r, value, r),
                (Some(r), Some(b), None) => (value, r, b, r),
                (Some(r), Some(b), Some(l)) => (value, r, b, l),
            };
            let v = exec_eval(&cmds::set_padding(t, ri, bo, le)).await?; print_result(&v, json); Ok(())
        }
        Commands::Gap { value } => { let v = exec_eval(&cmds::set_gap(value)).await?; print_result(&v, json); Ok(()) }
        Commands::Align { alignment } => { let v = exec_eval(&cmds::align(&alignment)).await?; print_result(&v, json); Ok(()) }
        Commands::A11y { action } => cmd_a11y(action, json).await,
        Commands::NodeTree { node_id, depth } => { let v = exec_eval(&cmds::node_tree(node_id.as_deref(), depth)).await?; print_result(&v, json); Ok(()) }
        Commands::NodeBindings { node_id } => { let v = exec_eval(&cmds::node_bindings(node_id.as_deref())).await?; print_result(&v, json); Ok(()) }
        Commands::Lint => { let v = exec_eval(cmds::lint()).await?; print_result(&v, json); Ok(()) }
        Commands::Analyze { action } => {
            let code = match action {
                AnalyzeAction::Colors => cmds::analyze_colors(),
                AnalyzeAction::Typography => cmds::analyze_typography(),
                AnalyzeAction::Spacing => cmds::analyze_spacing(),
                AnalyzeAction::Clusters => cmds::analyze_clusters(),
            };
            let v = exec_eval(code).await?; print_result(&v, json); Ok(())
        }
        Commands::Slot { action } => {
            let code = match action {
                SlotAction::Create { name, flex, gap, padding } => cmds::slot_create(&name, &flex, gap, padding),
                SlotAction::List { node_id } => cmds::slot_list(node_id.as_deref()),
                SlotAction::Reset { node_id } => cmds::slot_reset(node_id.as_deref()),
                SlotAction::Convert { node_id, name } => cmds::slot_convert(node_id.as_deref(), &name),
            };
            let v = exec_eval(&code).await?; print_result(&v, json); Ok(())
        }
        Commands::Variants { action } => {
            let code = match action {
                VariantsAction::From { ids, property, values, name } => {
                    let id_arr: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    let val_arr: Vec<String> = values.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    if id_arr.len() < 2 { anyhow::bail!("need at least 2 ids to create a variant set"); }
                    if id_arr.len() != val_arr.len() { anyhow::bail!("id count ({}) must equal values count ({})", id_arr.len(), val_arr.len()); }
                    cmds::variants_from(&id_arr, &property, &val_arr, &name)
                }
            };
            let v = exec_eval(&code).await?; print_result(&v, json); Ok(())
        }
        Commands::Prop { action } => {
            let code = match action {
                PropAction::Combine { ids, name } => {
                    let id_arr: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    cmds::prop_combine(&id_arr, &name)
                }
            };
            let v = exec_eval(&code).await?; print_result(&v, json); Ok(())
        }
        Commands::DeleteBatch { node_ids } => {
            // Accept comma-separated ids or a JSON array.
            let ids: Vec<String> = serde_json::from_str::<Vec<String>>(&node_ids)
                .unwrap_or_else(|_| node_ids.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
            let v = exec_eval(&cmds::delete_batch(&serde_json::to_string(&ids).unwrap())).await?;
            print_result(&v, json); Ok(())
        }
        Commands::Gradient { action } => cmd_gradient(action, json).await,
        Commands::Shadcn { action } => cmd_shadcn(action, json).await,
        Commands::Extract { args } => passthrough_cmd("extract", args).await,
        Commands::Import { args } => passthrough_cmd("import", args).await,
        Commands::Spec { args } => passthrough_cmd("spec", args).await,
        Commands::Instantiate { args } => passthrough_cmd("instantiate", args).await,
        Commands::Blocks { args } => passthrough_cmd("blocks", args).await,
        Commands::RecreateUrl { args } => passthrough_cmd("recreate-url", args).await,
        Commands::ScreenshotUrl { args } => passthrough_cmd("screenshot-url", args).await,
        Commands::RemoveBg { args } => passthrough_cmd("remove-bg", args).await,
        Commands::Dev { args } => passthrough_cmd("dev", args).await,
        Commands::Section { args } => passthrough_cmd("section", args).await,
        Commands::Grid { args } => passthrough_cmd("grid", args).await,
        Commands::Annotate { args } => passthrough_cmd("annotate", args).await,
        Commands::Plugins { args } => passthrough_cmd("plugins", args).await,
        Commands::Api { args } => passthrough_cmd("api", args).await,
        Commands::Sizes { args } => passthrough_cmd("sizes", args).await,
        Commands::Combos { args } => passthrough_cmd("combos", args).await,
        Commands::Js { args } => {
            // Raw forward: caller supplies the full Node CLI argument list.
            let code = passthrough::run(&args)?;
            if code != 0 { std::process::exit(code); }
            Ok(())
        }
    }
}

/// Ensure the daemon is up (so the Node CLI drives the same daemon + plugin),
/// then forward `<name> <args…>` to the Node fallback.
async fn passthrough_cmd(name: &str, args: Vec<String>) -> Result<()> {
    let _ = lifecycle::ensure_started().await;
    let mut full = vec![name.to_string()];
    full.extend(args);
    let code = passthrough::run(&full)?;
    if code != 0 { std::process::exit(code); }
    Ok(())
}

async fn cmd_shadcn(action: ShadcnAction, json: bool) -> Result<()> {
    match action {
        ShadcnAction::List => {
            let v: Value = serde_json::from_str(&tools::call("shadcnList", "{}")?)?;
            print_result(&v, json); Ok(())
        }
        ShadcnAction::Add { names, all, count } => {
            let args = serde_json::json!({ "names": names, "all": all, "count": count }).to_string();
            let out: Value = serde_json::from_str(&tools::call("shadcnAdd", &args)?)?;
            if let Some(err) = out.get("error").and_then(|e| e.as_str()) {
                anyhow::bail!(err.to_string());
            }
            // Each item is an independent component → render as a batch (independent nodes).
            let jsx: Vec<String> = out.get("items").and_then(|i| i.as_array())
                .map(|a| a.iter().filter_map(|it| it.get("jsx").and_then(|j| j.as_str()).map(String::from)).collect())
                .unwrap_or_default();
            if jsx.is_empty() { anyhow::bail!("no components selected (pass names or --all)"); }
            let req = transport::ExecRequest {
                action: "render-batch".into(),
                jsx_array: Some(jsx),
                gap: Some(40.0),
                vertical: Some(false),
                ..Default::default()
            };
            lifecycle::ensure_started().await?;
            let v = transport::exec(req).await?;
            save_last_render(&v);
            print_result(&v, json); Ok(())
        }
    }
}

async fn cmd_gradient(action: GradientAction, json: bool) -> Result<()> {
    match action {
        GradientAction::Extract { image, mode, apply_to, direction, stops, blur, no_trim } => {
            // Decode in Rust, analyze in the tools engine.
            let img = tools::decode_image(&image, 256)?;
            let trim = !no_trim;
            if mode == "mesh" {
                let args = serde_json::json!({ "img": img, "trim": trim, "blur": blur }).to_string();
                let recipe: Value = serde_json::from_str(&tools::call("meshExtract", &args)?)?;
                if let Some(id) = apply_to {
                    let v = exec_eval(&cmds::apply_mesh_wallpaper(Some(&id), &recipe, 0.0, 0.0, "")).await?;
                    print_result(&v, json);
                } else {
                    print_result(&recipe, json);
                }
            } else {
                let args = serde_json::json!({ "img": img, "direction": direction, "stops": stops, "trim": trim }).to_string();
                let out: Value = serde_json::from_str(&tools::call("gradientExtract", &args)?)?;
                if let Some(id) = apply_to {
                    let paint = out.get("paint").cloned().unwrap_or(Value::Null);
                    let v = exec_eval(&cmds::apply_paint(&id, &paint)).await?;
                    print_result(&v, json);
                } else {
                    print_result(&out, json);
                }
            }
            Ok(())
        }
        GradientAction::Mesh { colors, apply_to, base, size, blur, style, seed, name } => {
            let palette: Vec<String> = colors.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let args = serde_json::json!({ "colors": palette, "base": base, "blur": blur, "style": style, "seed": seed }).to_string();
            let recipe: Value = serde_json::from_str(&tools::call("meshFromColors", &args)?)?;
            // Apply to an existing frame, or create a new wallpaper frame.
            let (w, h) = size.split_once('x').and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?))).unwrap_or((1920.0, 1080.0));
            let code = cmds::apply_mesh_wallpaper(apply_to.as_deref(), &recipe, w, h, &name);
            let v = exec_eval(&code).await?;
            save_last_render(&v);
            print_result(&v, json);
            Ok(())
        }
    }
}

const A11Y_CONTRAST: &str = include_str!("../assets/cmd/a11y_contrast.js");
const A11Y_TOUCH: &str = include_str!("../assets/cmd/a11y_touch.js");
const A11Y_TEXT: &str = include_str!("../assets/cmd/a11y_text.js");

async fn cmd_a11y(action: A11yAction, json: bool) -> Result<()> {
    match action {
        A11yAction::Contrast { node_id, level } => {
            let v = run_asset(A11Y_CONTRAST, serde_json::json!({ "nodeId": node_id, "level": level.to_uppercase() })).await?;
            print_result(&v, json); Ok(())
        }
        A11yAction::Touch { node_id, min } => {
            let v = run_asset(A11Y_TOUCH, serde_json::json!({ "nodeId": node_id, "minSize": min })).await?;
            print_result(&v, json); Ok(())
        }
        A11yAction::Text { node_id } => {
            let v = run_asset(A11Y_TEXT, serde_json::json!({ "nodeId": node_id })).await?;
            print_result(&v, json); Ok(())
        }
        A11yAction::Audit { node_id } => {
            // Run all three checks and aggregate.
            let args = serde_json::json!({ "nodeId": node_id, "level": "AA", "minSize": 44 });
            let contrast = run_asset(A11Y_CONTRAST, args.clone()).await?;
            let touch = run_asset(A11Y_TOUCH, args.clone()).await?;
            let text = run_asset(A11Y_TEXT, args).await?;
            let v = serde_json::json!({ "contrast": contrast, "touch": touch, "text": text });
            print_result(&v, json); Ok(())
        }
    }
}

async fn cmd_create(action: CreateAction, json: bool) -> Result<()> {
    use cmds::*;
    let code = match action {
        CreateAction::Frame { name, width, height, x, y, fill, radius } =>
            create_frame(&ShapeOpts { name: Some(name), width, height: Some(height), x, y, fill, stroke: None, radius, opacity: None }),
        CreateAction::Rect { name, width, height, x, y, fill, stroke, radius, opacity } =>
            create_rect(&ShapeOpts { name, width, height: Some(height), x, y, fill, stroke, radius, opacity }),
        CreateAction::Ellipse { name, width, height, x, y, fill, stroke } =>
            create_ellipse(&ShapeOpts { name, width, height, x, y, fill, stroke, radius: None, opacity: None }),
        CreateAction::Text { content, x, y, size, color, weight, font, width, spacing } =>
            create_text(&TextOpts { content, x, y, size, color, weight, font, width, spacing }),
        CreateAction::Component { name } => create_component(name.as_deref()),
        CreateAction::Group { name } => create_group(name.as_deref()),
        CreateAction::Autolayout { name, direction, gap, padding, x, y, fill, radius, spacing } =>
            create_autolayout(&AutoLayoutOpts { name, direction, gap, padding, x, y, fill, radius, spacing }),
    };
    let v = exec_eval(&code).await?;
    save_last_render(&v);
    print_result(&v, json);
    Ok(())
}

async fn cmd_set(action: SetAction, json: bool) -> Result<()> {
    use cmds::Sel;
    let code = match action {
        SetAction::Fill { color, node, query } => cmds::set_fill(&Sel { node, query }, &color),
        SetAction::Stroke { color, node, query } => cmds::set_stroke(&Sel { node, query }, &color),
        SetAction::Radius { value, node, query } => cmds::set_radius(&Sel { node, query }, value),
        SetAction::Size { width, height, node, query } => cmds::set_size(&Sel { node, query }, width, height),
        SetAction::Scale { factor, node, query, keep_spacing } => cmds::set_scale(&Sel { node, query }, factor, keep_spacing),
        SetAction::Pos { x, y, node, query } => cmds::set_pos(&Sel { node, query }, x, y),
        SetAction::Opacity { value, node, query } => cmds::set_opacity(&Sel { node, query }, value),
        SetAction::Name { name, node, query } => cmds::set_name(&Sel { node, query }, &name),
        SetAction::Text { text, node, query } => cmds::set_text(&Sel { node, query }, &text),
    };
    let v = exec_eval(&code).await?;
    print_result(&v, json);
    Ok(())
}

async fn cmd_bind(action: BindAction, json: bool) -> Result<()> {
    use cmds::Sel;
    let code = match action {
        BindAction::Fill { var_name, node, query } => cmds::bind_fill(&Sel { node, query }, &var_name),
        BindAction::Stroke { var_name, node, query } => cmds::bind_stroke(&Sel { node, query }, &var_name),
        BindAction::Radius { var_name, node, query } => cmds::bind_prop(&Sel { node, query }, &var_name, "cornerRadius"),
        BindAction::Gap { var_name, node, query } => cmds::bind_prop(&Sel { node, query }, &var_name, "itemSpacing"),
        BindAction::Padding { var_name, node, query, side } => cmds::bind_padding(&Sel { node, query }, &var_name, &side),
    };
    let v = exec_eval(&code).await?;
    print_result(&v, json);
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 2: node ops, undo, variables, export
// ---------------------------------------------------------------------------

fn last_render_file() -> std::path::PathBuf {
    config::config_dir().join("last-render.json")
}

/// Recursively collect Figma node ids (strings matching N:M) under "id" keys.
fn collect_node_ids(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("id") {
                if id.split_once(':').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty()) {
                    out.push(id.clone());
                }
            }
            for (_, val) in map {
                collect_node_ids(val, out);
            }
        }
        Value::Array(arr) => arr.iter().for_each(|x| collect_node_ids(x, out)),
        _ => {}
    }
}

/// Persist ids created by a render so `undo` can remove them.
fn save_last_render(result: &Value) {
    let mut ids = Vec::new();
    collect_node_ids(result, &mut ids);
    if ids.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(config::config_dir());
    let _ = std::fs::write(last_render_file(), serde_json::json!({ "ids": ids }).to_string());
}

async fn cmd_node(action: NodeAction, json: bool) -> Result<()> {
    match action {
        NodeAction::ToComponent { node_ids } => {
            let ids = serde_json::to_string(&node_ids)?;
            let code = format!(
                "return (async () => {{ const ids = {ids}; const out = []; for (const id of ids) {{ const n = await figma.getNodeByIdAsync(id); if (n && (n.type==='FRAME'||n.type==='GROUP')) {{ const c = figma.createComponentFromNode(n); out.push({{id:c.id,name:c.name}}); }} }} return out; }})()"
            );
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        NodeAction::Delete { node_ids } => {
            let ids = serde_json::to_string(&node_ids)?;
            let code = format!(
                "return (async () => {{ const ids = {ids}; let deleted = 0; for (const id of ids) {{ const n = await figma.getNodeByIdAsync(id); if (n) {{ n.remove(); deleted++; }} }} return {{ deleted }}; }})()"
            );
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
    }
}

async fn cmd_unwrap(node_id: String, keep_wrapper: bool, json: bool) -> Result<()> {
    let id = js_string(&node_id);
    let code = format!(
        r#"return (async () => {{
  const n = await figma.getNodeByIdAsync({id});
  if (!n) throw new Error('Node not found: ' + {id});
  if (!('children' in n) || !Array.isArray(n.children)) return 'Node ' + n.id + ' has no children to unwrap';
  const parent = n.parent;
  if (!parent) throw new Error('Wrapper has no parent');
  const isOnPage = parent.type === 'PAGE';
  const offX = isOnPage ? n.x : 0, offY = isOnPage ? n.y : 0;
  const moved = [];
  for (const c of n.children.slice()) {{
    const cx = c.x, cy = c.y;
    parent.appendChild(c);
    if (isOnPage && 'x' in c) {{ c.x = offX + cx; c.y = offY + cy; }}
    moved.push(c.id);
  }}
  const wid = n.id, wname = n.name;
  if (!{keep_wrapper}) n.remove();
  return {{ unwrapped: wid, name: wname, children: moved, deletedWrapper: !{keep_wrapper} }};
}})()"#
    );
    let v = exec_eval(&code).await?;
    print_result(&v, json);
    Ok(())
}

async fn cmd_undo(json: bool) -> Result<()> {
    let path = last_render_file();
    let state: Value = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or(Value::Null),
        Err(_) => {
            if json { print_result(&serde_json::json!({ "removed": 0 }), true); } else { println!("Nothing to undo."); }
            return Ok(());
        }
    };
    let ids = state.get("ids").cloned().unwrap_or(Value::Array(vec![]));
    let code = format!(
        "return (async () => {{ let removed = 0; const names = []; for (const id of {ids}) {{ const n = await figma.getNodeByIdAsync(id); if (n && !n.removed) {{ names.push(n.name); n.remove(); removed++; }} }} return {{ removed, names }}; }})()"
    );
    let v = exec_eval(&code).await?;
    let _ = std::fs::remove_file(&path);
    print_result(&v, json);
    Ok(())
}

async fn cmd_var(action: VarAction, json: bool) -> Result<()> {
    match action {
        VarAction::List => {
            let code = r#"return (async () => {
  const cols = await figma.variables.getLocalVariableCollectionsAsync();
  const vars = await figma.variables.getLocalVariablesAsync();
  return cols.map(c => ({
    collection: c.name,
    modes: c.modes.map(m => m.name),
    variables: vars.filter(v => v.variableCollectionId === c.id).map(v => ({ name: v.name, type: v.resolvedType }))
  }));
})()"#.to_string();
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        VarAction::DeleteAll { collection } => {
            let filter = match &collection {
                Some(name) => format!("cols = cols.filter(c => c.name.includes({}));", js_string(name)),
                None => String::new(),
            };
            let code = format!(
                "return (async () => {{ let cols = await figma.variables.getLocalVariableCollectionsAsync(); {filter} let deleted = 0; for (const col of cols) {{ const vars = await figma.variables.getLocalVariablesAsync(); for (const v of vars.filter(v => v.variableCollectionId === col.id)) {{ v.remove(); deleted++; }} col.remove(); }} return {{ deletedVariables: deleted, deletedCollections: cols.length }}; }})()"
            );
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        VarAction::Create { name, collection, r#type, value } => {
            let code = cmds::var_create(&name, &collection, &r#type, value.as_deref());
            let v = exec_eval(&code).await?;
            print_result(&v, json);
            Ok(())
        }
        VarAction::Find { pattern } => {
            let v = exec_eval(&cmds::var_find(&pattern)).await?;
            print_result(&v, json);
            Ok(())
        }
        VarAction::Visualize { collection } => {
            let v = run_asset(VAR_VISUALIZE, serde_json::json!({ "collection": collection })).await?;
            print_result(&v, json); Ok(())
        }
        VarAction::CreateBatch { data: arr, collection } => {
            let parsed: Value = serde_json::from_str(&arr).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
            let v = exec_eval(&cmds::var_create_batch(&collection, &parsed.to_string())).await?;
            print_result(&v, json); Ok(())
        }
        VarAction::BindBatch { data: arr } => {
            let parsed: Value = serde_json::from_str(&arr).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
            let v = exec_eval(&cmds::bind_batch(&parsed.to_string())).await?;
            print_result(&v, json); Ok(())
        }
        VarAction::SetBatch { data: arr, collection } => {
            let parsed: Value = serde_json::from_str(&arr).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
            // Normalize id->nodeId, newName/label->name (LLMs reach for `id`).
            let ops: Vec<Value> = parsed.as_array().cloned().unwrap_or_default().into_iter().map(|mut o| {
                if let Some(map) = o.as_object_mut() {
                    if !map.contains_key("nodeId") { if let Some(id) = map.get("id").cloned() { map.insert("nodeId".into(), id); } }
                    if !map.contains_key("name") {
                        if let Some(n) = map.get("newName").or_else(|| map.get("label")).cloned() { map.insert("name".into(), n); }
                    }
                }
                o
            }).collect();
            let v = run_asset(SET_BATCH, serde_json::json!({ "operations": ops, "colFilter": collection })).await?;
            print_result(&v, json); Ok(())
        }
        VarAction::RenameBatch { data: arr } => {
            let parsed: Value = serde_json::from_str(&arr).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
            // Accept array [{id|nodeId, name|newName}] or object {id: name}.
            let pairs: Vec<Value> = match &parsed {
                Value::Array(a) => a.iter().map(|r| serde_json::json!({
                    "id": r.get("id").or_else(|| r.get("nodeId")),
                    "name": r.get("name").or_else(|| r.get("newName")),
                })).collect(),
                Value::Object(m) => m.iter().map(|(id, name)| serde_json::json!({ "id": id, "name": name })).collect(),
                _ => vec![],
            };
            let v = exec_eval(&cmds::rename_batch(&Value::Array(pairs).to_string())).await?;
            print_result(&v, json); Ok(())
        }
    }
}

const VAR_VISUALIZE: &str = include_str!("../assets/cmd/var_visualize.js");
const SET_BATCH: &str = include_str!("../assets/cmd/set_batch.js");

const TOKENS_SPACING: &str = include_str!("../assets/cmd/tokens_spacing.js");
const TOKENS_RADII: &str = include_str!("../assets/cmd/tokens_radii.js");
const EXPORT_CSS: &str = include_str!("../assets/cmd/export_css.js");
const EXPORT_TAILWIND: &str = include_str!("../assets/cmd/export_tailwind.js");
const EXPORT_DTCG: &str = include_str!("../assets/cmd/export_dtcg.js");

/// Wrap a param-free asset body (plain statements ending in `return …`) in an async IIFE.
fn wrap_iife(body: &str) -> String {
    format!("(async () => {{ {body} }})()")
}

// Local CLI config stored at ~/.figma-ds-cli/config.json.
fn cfg_path() -> std::path::PathBuf {
    config::config_dir().join("config.json")
}
fn cfg_load() -> serde_json::Map<String, Value> {
    std::fs::read_to_string(cfg_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}
fn cfg_set(key: &str, value: &str) -> Result<()> {
    let mut m = cfg_load();
    m.insert(key.to_string(), Value::String(value.to_string()));
    std::fs::create_dir_all(config::config_dir())?;
    std::fs::write(cfg_path(), serde_json::to_string_pretty(&Value::Object(m))?)?;
    Ok(())
}
fn cfg_get(key: &str) -> Result<Option<String>> {
    Ok(cfg_load().get(key).and_then(|v| v.as_str().map(String::from)))
}

async fn cmd_export(format: String, node_id: Option<String>, output: Option<String>, scale: f64, json: bool) -> Result<()> {
    let fmt = format.to_uppercase();
    let node_lookup = match &node_id {
        Some(id) => format!("node = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const sel = figma.currentPage.selection; node = sel.length > 0 ? sel[0] : null;".to_string(),
    };
    let settings = if fmt == "PNG" || fmt == "JPG" {
        format!("{{ format: {}, constraint: {{ type: 'SCALE', value: {scale} }} }}", js_string(&fmt))
    } else {
        format!("{{ format: {} }}", js_string(&fmt))
    };
    let code = format!(
        r#"return (async () => {{
  let node;
  {node_lookup}
  if (!node) return {{ error: 'No node selected or found' }};
  if (!('exportAsync' in node)) return {{ error: 'Node cannot be exported' }};
  const bytes = await node.exportAsync({settings});
  return {{ id: node.id, name: node.name, base64: figma.base64Encode(bytes) }};
}})()"#
    );
    let v = exec_eval(&code).await?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!(err.to_string());
    }
    let b64 = v.get("base64").and_then(|b| b.as_str()).unwrap_or("");
    let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("node").replace(':', "-");
    let ext = format.to_lowercase();
    let path = output.unwrap_or_else(|| format!("./export-{id}.{ext}"));
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| anyhow::anyhow!("decode export: {e}"))?;
    std::fs::write(&path, &bytes)?;
    let out = serde_json::json!({ "id": v.get("id"), "name": v.get("name"), "saved": path, "bytes": bytes.len() });
    if json { print_result(&out, true); } else { println!("✓ exported {} ({} bytes)", path, bytes.len()); }
    Ok(())
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

/// Run a bundled command-asset JS body with `__args` injected, via the plugin.
/// The asset body is plain statements ending in `return …`; we wrap it in an
/// async IIFE so the plugin evaluates and returns its value.
async fn run_asset(body: &str, args: Value) -> Result<Value> {
    let code = format!("(async () => {{ const __args = {}; {} }})()", args, body);
    exec_eval(&code).await
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
