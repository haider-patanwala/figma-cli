# figma-cli (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Figma-Desktop-purple" alt="Figma Desktop">
  <img src="https://img.shields.io/badge/No_API_Key-Required-green" alt="No API Key">
  <img src="https://img.shields.io/badge/Safe_Mode-Plugin_Bridge-blue" alt="Safe Mode">
  <img src="https://img.shields.io/badge/Single-Binary-black" alt="Single Binary">
</p>

A single-binary Rust port of [figma-ds-cli](../README.md). It controls **Figma
Desktop** directly through a local plugin bridge — **no API key, no cloud, no MCP**.
You (or an AI agent) describe what you want; it builds it live in Figma.

This is the **Safe Mode** implementation: it never patches the Figma app. A small
bundled plugin (**FigCli**) runs inside Figma and talks to the binary over a local
WebSocket.

## How it works

```
figma-cli <cmd>  ──HTTP :3456──▶  daemon  ──WebSocket /plugin──▶  FigCli plugin (in Figma)
                                  (Rust)                          evals via the figma.* API
```

- The CLI is short-lived: it POSTs to a long-lived **daemon** on `127.0.0.1:3456`.
- The daemon holds the plugin connection and dispatches work.
- **JSX → Plugin-API codegen** runs in an embedded **QuickJS** engine inside the
  daemon (the proven renderer is bundled, not re-derived).
- A second QuickJS **tools** engine runs bundled pure-JS for **gradient analysis**
  and the **shadcn** component library; `<Icon>` SVGs and source images are fetched/
  decoded in Rust.
- Node creation always happens **inside Figma** via the plugin — the binary just
  generates and ships the JS.

A handful of heavy/niche commands (DESIGN.md `extract`/`import`/`spec`/`instantiate`,
`blocks`, `recreate-url`/`screenshot-url`/`remove-bg`, `dev`/`section`/`grid`/
`annotate`/`plugins`/`api`/`sizes`/`combos`) **fall back to the original Node CLI**,
which speaks the daemon's exact protocol and drives the *same* daemon + plugin. Those
need Node 18+; everything else is pure Rust.

## Install

```bash
bash bin/install.sh          # from the skill package — installs to ~/.local/bin
# or build from source:
cargo build --release        # binary → target/release/figma-cli
```

## Quick start

```bash
figma-cli connect            # starts the daemon, prints how to import the FigCli plugin
# In Figma: Plugins → Development → Import plugin from manifest… → run FigCli, keep its window open
figma-cli connect            # → ✓ Connected to Figma (Safe Mode)

figma-cli render '<Frame bg="#3b82f6" px={16} py={10} rounded={10} flex="row" justify="center" items="center"><Text color="#fff">Click me</Text></Frame>'
figma-cli tokens preset shadcn
figma-cli verify --save /tmp/check.png
```

Add `--json` to any command for machine-readable output.

## Commands

57 top-level commands. See [skill/reference/REFERENCE.md](skill/reference/REFERENCE.md)
for the full list and [skill/reference/JSX.md](skill/reference/JSX.md) for the JSX
render syntax. Highlights:

| Area | Commands |
|------|----------|
| Connection | `connect`, `daemon start/status/stop/restart` |
| Render | `render`, `render-batch`, `eval` |
| Create/edit | `create …`, `set …`, `sizing …`, `bind …`, `select/delete/duplicate/get`, `unwrap`, `undo` |
| Tokens/vars | `tokens preset shadcn`, `tokens spacing/radii`, `var list/create/find/visualize/delete-all`, `var *-batch`, `export-tokens css/tailwind/dtcg` |
| Components | `shadcn list/add`, `variants from`, `prop combine`, `slot create/list/reset/convert` |
| Quality | `verify`, `a11y contrast/touch/text/audit`, `lint`, `analyze …`, `node-tree`, `node-bindings` |
| Visuals | `gradient extract`, `gradient mesh`, `export png/svg/jpg/pdf` |
| FigJam | `figjam sticky/shape/text/connect/move/update/delete/info/nodes` |
| Node-fallback | `extract`, `import`, `spec`, `instantiate`, `blocks`, `recreate-url`, `screenshot-url`, `remove-bg`, `dev`, `section`, `grid`, `annotate`, `plugins`, `api`, `sizes`, `combos`, `js <args…>` |

## Development

```
src/
  main.rs        # clap CLI + dispatch
  daemon.rs      # axum HTTP + WS /plugin bridge (protocol-compatible with the JS daemon)
  engine.rs      # QuickJS render engine (assets/engine.js)
  tools.rs       # QuickJS tools engine + Rust image decode (assets/tools.js)
  cmds.rs        # JS-payload builders for eval-based commands
  jsgen.rs       # pure JS-string helpers (fills, var loading, node selectors)
  passthrough.rs # Node fallback for heavy/niche commands
  lifecycle.rs   # daemon spawn/status/stop
  transport.rs   # CLI → daemon HTTP client
  config.rs      # paths, port, auth token
assets/
  engine.js      # bundled JSX→Plugin-API codegen   (build: engine-src/build-engine.sh)
  tools.js       # bundled gradient + shadcn         (build: tools-src/build-tools.sh)
  cmd/*.js       # large parameterized eval payloads (a11y, var visualize, set-batch, …)
  plugin/        # FigCli manifest.json / code.js / ui.html (embedded, written on `connect`)
```

### Self-contained

The `rust/` directory is self-contained — **building and running the binary need
nothing from the repo root.** The JS engines (`assets/engine.js`, `assets/tools.js`)
are committed and embedded via `include_str!`, the FigCli plugin is embedded, and
the Node fallback ships vendored under `skill/js/` (its source is also the input the
build scripts read). Verified by building + running with the repo root removed.

Regenerating the bundled engines (only needed when the vendored JS in `skill/js/src`
changes) requires `esbuild`:

```bash
bash engine-src/build-engine.sh && bash tools-src/build-tools.sh && cargo build --release
# esbuild: `npm i -g esbuild` or run from a dir where it's available
```

## License

MIT.
