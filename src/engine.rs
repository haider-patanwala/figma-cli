//! JSX -> Figma Plugin API codegen, executed by the bundled JS engine.
//!
//! Per the project plan the proven codegen (`FigmaClient.parseJSX` /
//! `parseJSXBatch` from `src/figma-client.js`) is NOT rewritten in Rust — it is
//! esbuild-bundled into `assets/engine.js` and run inside an embedded QuickJS
//! runtime. The only impure step, fetching `<Icon>` SVGs from the Iconify API,
//! is done here in Rust (blocking reqwest) and passed into the engine as a map,
//! keeping the JS side fully synchronous.
//!
//! QuickJS values are `!Send`, so the runtime lives on a dedicated OS thread
//! and is driven through a channel.

use anyhow::{anyhow, bail, Result};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;

const ENGINE_JS: &str = include_str!("../assets/engine.js");

pub struct RenderOpts {
    pub gap: f64,
    pub vertical: bool,
    pub collection: Option<String>,
}

enum Req {
    ParseOne {
        jsx: String,
        collection: Option<String>,
        resp: Sender<Result<String, String>>,
    },
    ParseBatch {
        jsx_array: Vec<String>,
        opts_gap: f64,
        opts_vertical: bool,
        collection: Option<String>,
        resp: Sender<Result<String, String>>,
    },
}

static ENGINE: OnceLock<Sender<Req>> = OnceLock::new();

fn engine() -> &'static Sender<Req> {
    ENGINE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Req>();
        std::thread::Builder::new()
            .name("figma-engine".into())
            .spawn(move || engine_thread(rx))
            .expect("spawn engine thread");
        tx
    })
}

/// Convert a single JSX string into a Plugin API JS payload.
pub fn parse_jsx(jsx: &str, collection: Option<&str>) -> Result<String> {
    let (resp, rx) = mpsc::channel();
    engine()
        .send(Req::ParseOne {
            jsx: jsx.to_string(),
            collection: collection.map(String::from),
            resp,
        })
        .map_err(|_| anyhow!("engine thread unavailable"))?;
    rx.recv().map_err(|_| anyhow!("engine thread dropped"))?.map_err(|e| anyhow!(e))
}

/// Convert an array of JSX strings into a single batched JS payload.
pub fn parse_jsx_batch(jsx: &[String], opts: &RenderOpts) -> Result<String> {
    let (resp, rx) = mpsc::channel();
    engine()
        .send(Req::ParseBatch {
            jsx_array: jsx.to_vec(),
            opts_gap: opts.gap,
            opts_vertical: opts.vertical,
            collection: opts.collection.clone(),
            resp,
        })
        .map_err(|_| anyhow!("engine thread unavailable"))?;
    rx.recv().map_err(|_| anyhow!("engine thread dropped"))?.map_err(|e| anyhow!(e))
}

/// Generate the JS payload that creates a named token preset (e.g. shadcn).
pub fn tokens_preset(name: &str) -> Result<String> {
    const SHADCN_TOKENS_JS: &str = include_str!("../assets/tokens-shadcn.js");
    match name {
        "shadcn" => Ok(SHADCN_TOKENS_JS.to_string()),
        other => bail!("unknown token preset '{other}' (available: shadcn)"),
    }
}

// ---------------------------------------------------------------------------
// Engine thread (owns the QuickJS context)
// ---------------------------------------------------------------------------

fn engine_thread(rx: mpsc::Receiver<Req>) {
    use rquickjs::{Context, Runtime};

    let rt = Runtime::new().expect("quickjs runtime");
    let ctx = Context::full(&rt).expect("quickjs context");

    // Console shim (engine calls console.warn on unparsed children) + load engine.
    let init = ctx.with(|ctx| -> std::result::Result<(), String> {
        ctx.eval::<(), _>(
            "globalThis.console = { log(){}, warn(){}, error(){}, info(){}, debug(){} };",
        )
        .map_err(|e| format!("console shim: {e}"))?;
        ctx.eval::<(), _>(ENGINE_JS).map_err(|e| format!("load engine.js: {e}"))?;
        Ok(())
    });
    if let Err(e) = init {
        // Drain requests with the init error so callers fail loudly.
        let msg = format!("engine init failed: {e}");
        for req in rx {
            reply_err(req, &msg);
        }
        return;
    }

    for req in rx {
        match req {
            Req::ParseOne { jsx, collection, resp } => {
                let r = run_parse_one(&ctx, &jsx, collection.as_deref());
                let _ = resp.send(r.map_err(|e| e.to_string()));
            }
            Req::ParseBatch { jsx_array, opts_gap, opts_vertical, collection, resp } => {
                let r = run_parse_batch(&ctx, &jsx_array, opts_gap, opts_vertical, collection.as_deref());
                let _ = resp.send(r.map_err(|e| e.to_string()));
            }
        }
    }
}

fn reply_err(req: Req, msg: &str) {
    match req {
        Req::ParseOne { resp, .. } => { let _ = resp.send(Err(msg.to_string())); }
        Req::ParseBatch { resp, .. } => { let _ = resp.send(Err(msg.to_string())); }
    }
}

/// Call __engine.iconNames(jsxArrayJson) -> Vec<String>.
fn collect_icon_names(ctx: &rquickjs::Context, jsx_array: &[String]) -> Result<Vec<String>> {
    use rquickjs::{Function, Object};
    let jsx_json = serde_json::to_string(jsx_array)?;
    let names_json: String = ctx
        .with(|ctx| -> std::result::Result<String, String> {
            let engine: Object = ctx.globals().get("__engine").map_err(|e| e.to_string())?;
            let f: Function = engine.get("iconNames").map_err(|e| e.to_string())?;
            f.call((jsx_json,)).map_err(|e| e.to_string())
        })
        .map_err(|e| anyhow!(e))?;
    Ok(serde_json::from_str(&names_json)?)
}

/// Fetch SVGs for the named icons from Iconify; failures fall back to omission
/// (the engine renders a placeholder box when an icon is missing).
fn fetch_icons(names: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for name in names {
        if let Some((prefix, icon)) = name.split_once(':') {
            let url = format!("https://api.iconify.design/{prefix}/{icon}.svg?width=24&height=24");
            if let Ok(resp) = reqwest::blocking::get(&url) {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text() {
                        map.insert(name.clone(), text);
                    }
                }
            }
        }
    }
    map
}

fn run_parse_one(ctx: &rquickjs::Context, jsx: &str, collection: Option<&str>) -> Result<String> {
    use rquickjs::{Function, Object};
    let names = collect_icon_names(ctx, std::slice::from_ref(&jsx.to_string()))?;
    let icon_json = serde_json::to_string(&fetch_icons(&names))?;
    let col = collection.unwrap_or("").to_string();
    ctx.with(|ctx| -> std::result::Result<String, String> {
        let engine: Object = ctx.globals().get("__engine").map_err(|e| e.to_string())?;
        let f: Function = engine.get("parseOne").map_err(|e| e.to_string())?;
        f.call((jsx.to_string(), icon_json, col)).map_err(|e| e.to_string())
    })
    .map_err(|e| anyhow!(e))
}

fn run_parse_batch(
    ctx: &rquickjs::Context,
    jsx_array: &[String],
    gap: f64,
    vertical: bool,
    collection: Option<&str>,
) -> Result<String> {
    use rquickjs::{Function, Object, Promise};
    let names = collect_icon_names(ctx, jsx_array)?;
    let icon_json = serde_json::to_string(&fetch_icons(&names))?;
    let jsx_json = serde_json::to_string(jsx_array)?;
    let col = collection.unwrap_or("").to_string();
    ctx.with(|ctx| -> std::result::Result<String, String> {
        let engine: Object = ctx.globals().get("__engine").map_err(|e| e.to_string())?;
        let f: Function = engine.get("parseBatch").map_err(|e| e.to_string())?;
        let promise: Promise = f
            .call((jsx_json, icon_json, gap, vertical, col))
            .map_err(|e| e.to_string())?;
        // parseJSXBatch resolves synchronously; finish drives pending jobs.
        promise.finish::<String>().map_err(|e| e.to_string())
    })
    .map_err(|e| anyhow!(e))
}
