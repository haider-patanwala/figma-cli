# Decisions & rationale

Each entry: the decision, why, and what it implies for future work.

## D1 — Safe Mode (plugin bridge) only; never patch Figma
**Why:** explicit user constraint. The original tool's "Yolo mode" byte-patches Figma's
`app.asar` to open a CDP debug port; we don't ship that.
**Implies:** the daemon is a WebSocket *server*; the plugin is the client. There is no
CDP path in the Rust binary. The user must import/run the FigCli plugin once.

## D2 — Bundle the proven JS codegen; don't rewrite it in Rust
**Why:** user constraint ("bundle the JS engine, port the shell") + risk. The JSX→Plugin-
API renderer is ~5k lines of subtle layout logic; re-deriving it in Rust would diverge.
**Implies:** Rust owns the *shell* (CLI, transport, daemon, lifecycle) and the impure
bits (HTTP, image decode); the codegen stays JS, run in embedded QuickJS. The bundles
are committed so `cargo build` needs no JS toolchain.

## D3 — QuickJS (`rquickjs`) for the embedded JS, not V8/Node
**Why:** small, embeddable, single-binary friendly; the codegen is pure string transform
so it doesn't need Node APIs. V8/deno_core would bloat the binary; shelling to Node would
add a runtime dependency for the core path.
**Implies:** no `fetch`/`fs` in the engines — Rust provides those (Iconify fetch, image
decode, file I/O) and injects results. Engines run on dedicated threads (QuickJS is `!Send`).
Async JS (parseJSXBatch) is resolved with `Promise::finish`.

## D4 — Two engines (render + tools), both string-FFI
**Why:** separation; the tools engine (gradient/shadcn) has different deps than the render
engine. All FFI is JSON strings in/out to keep marshalling trivial.
**Implies:** to add a bundled-JS feature, add it to the relevant `*-src/entry.mjs`, expose
a function returning JSON, rebuild the bundle, call via `engine::`/`tools::call`.

## D5 — Image decoding in Rust (`image` crate), pixels injected
**Why:** pngjs/jpeg-js don't run in QuickJS (Buffer/zlib). `image` handles PNG/JPEG/WebP/GIF.
We downscale to ≤256px before injecting — the gradient algorithm samples anyway, and it keeps
QuickJS marshalling cheap.
**Implies:** `gradient extract` is pure Rust decode + bundled-JS analysis. Faithful and
single-binary.

## D6 — Node fallback for the heavy/niche tail, instead of porting everything
**Why:** user authorized "use Rust if possible, else a wrapper." DESIGN.md extract/import/
spec/instantiate (~1.5k lines of markdown + YAML), code-import (evaluates tailwind.config JS),
url-tools (headless browser), and the low-traffic misc groups would each be a sub-project with
real divergence risk. The original Node CLI already speaks our daemon's exact protocol.
**Implies:** those commands forward to `skill/js` (the vendored Node CLI) via `passthrough.rs`.
They need Node 18+. If a future agent wants them in pure Rust, the bundle/asset patterns (D4,
or `run_asset`) are the path — start with `spec` (pure file parse) and `import` (parse → eval).

## D7 — Protocol byte-compatibility with the original daemon/plugin
**Why:** lets the unmodified FigCli plugin connect AND lets the Node fallback drive our daemon.
**Implies:** do not change the `/exec`/`/health` shapes, the `x-daemon-token` header, the WS
message types, or port 3456 without updating both ends. The plugin assets are reused verbatim.

## D8 — `--json` global flag; results are plain JSON values
**Why:** agents consume output. Human mode pretty-prints; `--json` emits raw.
**Gotcha:** a clap field literally named `json` collides with the global `--json` flag — name
positional JSON args `data`/`json_array`, never `json` (this bit us once on the var batch ops).

## D9 — `undo` via persisted render ids (not Figma's native undo)
**Why:** the Plugin API doesn't expose a reliable programmatic undo. render/render-batch/create
write created node ids to `~/.figma-ds-cli/last-render.json`; `undo` removes them.
**Implies:** only the *last* create/render is undoable.

## D10 — Skill package moved into `rust/skill`; crate is self-contained
**Why:** the skill is the shippable artifact (skills.sh) and should be version-controlled with
the binary. The user also required `rust/` to have no repo-root dependency to build/run.
**Implies:** build scripts read vendored source from `skill/js/src` (not `../../src`); the
passthrough dev path points inside the crate. Verified by building+running with the repo root
removed. Only *regenerating* the JS bundles needs the `esbuild` tool (bundles are committed, so
this is optional). `skill/js/node_modules` is gitignored (installed by `install.sh` at install time).

## D11 — Testing without Figma: fake plugin + syntax check
**Why:** validate the bridge and codegen offline/in CI.
**How:** `test-helpers/fake-plugin.mjs` speaks the plugin WS protocol and echoes canned results;
generated payloads are syntax-checked with `node new Function(code)`. Live Figma testing is the
final gate (done this session — see SESSION-LOG.md).
