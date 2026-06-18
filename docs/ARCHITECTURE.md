# Architecture

## The big picture

```
figma-cli <cmd>        Rust daemon (long-lived)              FigCli plugin (in Figma)
  HTTP POST /exec ───▶  HTTP server :3456                     WS client → ws://127.0.0.1:3456/plugin
  {action,code|jsx…}    - render/render-batch: JSX→JS         - eval()s JS with the global `figma`
                          via embedded QuickJS engine         - returns {type:result|batch-result,…}
  ◀── JSON {result}     - forwards {action:eval,id,code}
                          over WS to plugin ◀──────────────▶  ping/pong keepalive
```

Three processes. **Node creation always happens inside Figma** via the plugin — the
binary's job is to generate the right Plugin-API JS and ship it over the bridge.

This is **Safe Mode**: no patching of Figma's `app.asar`. The original Node tool also
had a "Yolo mode" (byte-patch Figma to open a CDP debug port) — we deliberately did
NOT port that (see DECISIONS.md).

## Processes & responsibilities

- **CLI** (`src/main.rs`, `src/transport.rs`): parse args (clap), POST to the daemon,
  print results. Short-lived. Auto-starts the daemon if needed (`src/lifecycle.rs`).
- **Daemon** (`src/daemon.rs`): `axum` HTTP server + WebSocket server sharing one
  listener on `127.0.0.1:3456`. Holds the single plugin connection, keeps a pending-
  request map (id → oneshot), dispatches `eval`/`eval-batch`, relays results. Runs via
  the hidden `figma-cli daemon-run` subcommand (spawned detached by `lifecycle`).
- **FigCli plugin** (`assets/plugin/{manifest.json,code.js,ui.html}`): unchanged from
  the original tool. The UI scans ports 3456–3460, connects, and relays eval requests
  to the plugin main thread which runs them against `figma`.

## Wire protocol (must stay byte-compatible with the original plugin)

**CLI ↔ daemon (HTTP, token-guarded):**
- `POST /exec` body `{action, code|jsx|jsxArray, gap, vertical, collection}` →
  `{result, mode}` or `{error}`.
- `GET /health` → `{status, mode:"safe", plugin:bool}`.
- Auth: header `x-daemon-token` must match `~/.figma-ds-cli/.daemon-token`; Host header
  must be localhost/127.0.0.1 (DNS-rebind guard). Mirrors the JS daemon.

**daemon ↔ plugin (WebSocket at `/plugin`):**
- daemon→plugin: `{action:"eval",id,code}` | `{action:"eval-batch",id,codes}` | `{type:"pong"}`
- plugin→daemon: `{type:"hello",version}` | `{type:"ping"}` | `{type:"result",id,result,error}`
  | `{type:"batch-result",id,results}`
- Timeouts: 25 s eval, 60 s batch (match the JS daemon).

Because this matches the original exactly, the **unmodified** FigCli plugin connects,
AND the original Node CLI (used for the fallback) drives our daemon transparently.

## The two embedded JS engines (QuickJS via `rquickjs`)

QuickJS values are `!Send`, so each engine runs on its own dedicated OS thread and is
driven through a channel.

1. **Render engine** (`src/engine.rs`, `assets/engine.js`):
   The proven `FigmaClient.parseJSX` / `parseJSXBatch` codegen, esbuild-bundled. Turns
   JSX strings into a Plugin-API JS payload. The only impurity — fetching `<Icon>` SVGs
   from the Iconify API — is done in **Rust** (`reqwest`), and the SVG map is injected,
   keeping the JS synchronous. Batch path returns a Promise resolved via `Promise::finish`.
   Built by `engine-src/build-engine.sh` (stubs `ws` + `./figma-patch.js`).

2. **Tools engine** (`src/tools.rs`, `assets/tools.js`):
   Bundles the pure-JS gradient analyzer (`gradient-extractor.js`) and the shadcn/ui
   component library (`shadcn.js`). Image decoding for `gradient extract` is done in
   **Rust** (`image` crate: decode + downscale to ≤256px), pixels injected as
   `opts.__img`. Built by `tools-src/build-tools.sh`.

Both bundles are committed in `assets/`, so `cargo build` needs no JS toolchain.

## Command implementation patterns

There are three ways a command is implemented; pick by what it needs:

1. **Inline JS codegen** (`src/cmds.rs` + `src/jsgen.rs`): a Rust fn builds a Plugin-API
   JS string, sent via `exec_eval`. Used for create/set/sizing/bind/node-ops/slots/
   variants/figjam/analyze, etc. `jsgen.rs` ports the pure helpers from the original
   `cli-core.js` (hex→rgb, fill/stroke code, var-loading, smart positioning, node
   selectors).
2. **Asset payloads** (`assets/cmd/*.js` + `run_asset(body, args)`): large self-contained
   eval payloads copied verbatim from the original, parameterized via a `__args` object
   the Rust side injects. Used for a11y contrast/touch/text, `var visualize`, `set-batch`,
   `tokens spacing/radii`, `export-tokens css/tailwind/dtcg`. Keeps fidelity for big payloads.
3. **Engine/tools call**: render/render-batch (render engine), gradient/shadcn (tools engine).
4. **Node fallback** (`src/passthrough.rs`): forward to the bundled Node CLI for the
   heavy/niche commands. The Node CLI talks to the same daemon, so behavior is identical.

## Node fallback

`src/passthrough.rs` forwards `extract`, `import`, `spec`, `instantiate`, `blocks`,
`recreate-url`, `screenshot-url`, `remove-bg`, `dev`, `section`, `grid`, `annotate`,
`plugins`, `api`, `sizes`, `combos`, and the `js <args…>` escape hatch to the original
Node CLI. It locates the CLI via `$FIGMA_CLI_JS`, `js/index.js` next to the binary
(skill install layout), or `skill/js/index.js` within the crate (dev). Requires Node 18+.
These were not reimplemented in Rust because they carry YAML parsing, headless-browser
screenshotting, large DESIGN.md round-trips, or are low-traffic — a faithful wrapper was
the better trade (see DECISIONS.md).

## File map (rust/)

```
src/
  main.rs        clap CLI definitions + dispatch (one match arm per command)
  daemon.rs      HTTP + WS /plugin bridge, pending-request map, auth
  lifecycle.rs   daemon spawn/status/stop/restart, pidfile, health poll
  transport.rs   CLI → daemon HTTP client (ExecRequest, exec(), health())
  config.rs      paths (port 3456, pidfile, token), token gen/read
  engine.rs      render QuickJS engine (parse_jsx / parse_jsx_batch / tokens_preset)
  tools.rs       tools QuickJS engine (gradient/shadcn) + decode_image (image crate)
  cmds.rs        JS-payload builders for eval-based commands
  jsgen.rs       pure JS-string helpers ported from cli-core.js
  passthrough.rs Node fallback
assets/
  engine.js      bundled render engine        (gen: engine-src/build-engine.sh)
  tools.js       bundled gradient + shadcn     (gen: tools-src/build-tools.sh)
  tokens-shadcn.js  shadcn token-collection creation payload
  cmd/*.js       parameterized eval payloads (a11y_*, var_visualize, set_batch, tokens_*, export_*)
  plugin/        FigCli manifest.json/code.js/ui.html (embedded; written on `connect`)
engine-src/      esbuild inputs for engine.js (entry.mjs + stubs; figma-client.js vendored-copy is gitignored)
tools-src/       esbuild inputs for tools.js (entry.mjs + stubs; gradient-extractor.js/shadcn.js copies gitignored)
test-helpers/    fake-plugin.mjs — simulates the plugin's WS protocol for offline testing
skill/           the installable skills.sh package (SKILL.md, reference/, bin/install.sh, prebuilt binary, plugin/, js/ Node fallback)
docs/            this folder
```
