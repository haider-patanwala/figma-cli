//! "Tools" QuickJS engine: runs the bundled pure-JS modules (gradient analysis,
//! and future design-md / code-import / renderers) that are too large to
//! re-derive in Rust. Image decoding is done here in Rust (`image` crate) and
//! injected as pixels; the JS does the analysis.
//!
//! Like the render engine, the QuickJS context is `!Send`, so it lives on a
//! dedicated thread driven through a channel.

use anyhow::{anyhow, Result};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;

const TOOLS_JS: &str = include_str!("../assets/tools.js");

struct Req {
    func: String,
    args_json: String,
    resp: Sender<Result<String, String>>,
}

static TOOLS: OnceLock<Sender<Req>> = OnceLock::new();

fn tools() -> &'static Sender<Req> {
    TOOLS.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Req>();
        std::thread::Builder::new()
            .name("figma-tools".into())
            .spawn(move || tools_thread(rx))
            .expect("spawn tools thread");
        tx
    })
}

/// Call `globalThis.__tools[func](args_json)` and return its string result.
pub fn call(func: &str, args_json: &str) -> Result<String> {
    let (resp, rx) = mpsc::channel();
    tools()
        .send(Req { func: func.to_string(), args_json: args_json.to_string(), resp })
        .map_err(|_| anyhow!("tools engine unavailable"))?;
    rx.recv().map_err(|_| anyhow!("tools engine dropped"))?.map_err(|e| anyhow!(e))
}

fn tools_thread(rx: mpsc::Receiver<Req>) {
    use rquickjs::{Context, Function, Object, Runtime};
    let rt = Runtime::new().expect("quickjs runtime");
    let ctx = Context::full(&rt).expect("quickjs context");
    let init = ctx.with(|ctx| -> std::result::Result<(), String> {
        ctx.eval::<(), _>("globalThis.console = { log(){}, warn(){}, error(){} };")
            .map_err(|e| format!("console shim: {e}"))?;
        ctx.eval::<(), _>(TOOLS_JS).map_err(|e| format!("load tools.js: {e}"))?;
        Ok(())
    });
    if let Err(e) = init {
        let msg = format!("tools init failed: {e}");
        for req in rx { let _ = req.resp.send(Err(msg.clone())); }
        return;
    }
    for req in rx {
        let r = ctx.with(|ctx| -> std::result::Result<String, String> {
            let tools: Object = ctx.globals().get("__tools").map_err(|e| e.to_string())?;
            let f: Function = tools.get(req.func.as_str()).map_err(|e| e.to_string())?;
            f.call((req.args_json.clone(),)).map_err(|e| e.to_string())
        });
        let _ = req.resp.send(r);
    }
}

/// Decode an image file and downscale to <= `max_dim` on its longest side,
/// returning a JSON value { width, height, data: [r,g,b,a, ...] } for injection.
pub fn decode_image(path: &str, max_dim: u32) -> Result<serde_json::Value> {
    let expanded = if let Some(rest) = path.strip_prefix('~') {
        format!("{}{}", dirs::home_dir().unwrap_or_default().display(), rest)
    } else {
        path.to_string()
    };
    let img = image::open(&expanded).map_err(|e| anyhow!("could not open image {expanded}: {e}"))?;
    let img = if img.width().max(img.height()) > max_dim {
        img.thumbnail(max_dim, max_dim)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let data: Vec<u32> = rgba.into_raw().into_iter().map(|b| b as u32).collect();
    Ok(serde_json::json!({ "width": w, "height": h, "data": data }))
}
