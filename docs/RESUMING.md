# Resuming work

You are picking up a Rust port of `figma-cli`. Read this, then skim
[ARCHITECTURE.md](ARCHITECTURE.md) and [DECISIONS.md](DECISIONS.md).

## Where things stand

- **Status:** functionally complete. ~57 top-level commands at parity with the original
  Node `figma-ds-cli`. Builds clean; tested live against real Figma this session.
- **`rust/` is self-contained** — building and running depend on nothing outside `rust/`.
- Two implementation tiers: **Rust-native** (the bulk) and **Node fallback** (heavy/niche).

## Build & run

```bash
cd rust
cargo build --release                 # → target/release/figma-cli  (no JS toolchain needed)
./target/release/figma-cli connect    # start daemon; follow prompt to import FigCli plugin in Figma
./target/release/figma-cli daemon status   # "plugin": true  ⇒ connected
```

If you change the bundled JS (in `skill/js/src`), regenerate the engines (needs `esbuild`):
```bash
bash engine-src/build-engine.sh && bash tools-src/build-tools.sh && cargo build --release
```

## Test loop

- **Offline (no Figma):** start the daemon, run `node test-helpers/fake-plugin.mjs &` (it
  speaks the plugin WS protocol and echoes canned results), then run commands. To validate a
  generated payload is well-formed JS, capture it from the fake and run `node -e 'new Function(code)'`.
  IMPORTANT: kill stale fake-plugin processes between runs — the daemon keeps only the
  *last*-connected plugin, so a leftover fake answers with wrong results.
- **Live (with Figma + FigCli):** the real gate. `verify <id> --save /tmp/x.png` then read the PNG.

## How to add a command (the patterns)

Pick the lightest tier that fits (see ARCHITECTURE.md §"Command implementation patterns"):

1. **Simple eval codegen** → add a builder in `src/cmds.rs` returning a Plugin-API JS string
   (use `jsgen.rs` helpers for fills/vars/selectors), add a clap variant in `src/main.rs`, and a
   dispatch arm calling `exec_eval(&code)`.
2. **Large/verbatim payload** → drop the JS in `assets/cmd/<name>.js` (plain statements ending in
   `return …`, params read from `__args`), `include_str!` it, call `run_asset(BODY, json!({…}))`.
3. **Needs bundled JS lib** → add it to `engine-src/` or `tools-src/` entry, expose a JSON-returning
   function, rebuild the bundle, call via `engine::` / `tools::call("fn", argsJson)`.
4. **Heavy/niche** → forward to Node: add a clap variant and a `passthrough_cmd("name", args)` arm.

**Gotcha:** never name a positional clap field `json` — it collides with the global `--json` flag
(use `data`, `json_array`, etc.).

## Suggested next work (in priority order)

The Node-fallback commands work, but if the goal is *pure Rust* parity:
1. **`spec`** — pure DESIGN.md file parse + print; no Figma. Bundle `design-spec.js` into the tools
   engine (Rust reads the file, passes content). Lowest risk.
2. **`import`** — parse DESIGN.md/tailwind.config/css/tokens.json → emit a variable-creation eval.
   Bundle `code-import/*` + `design-md.js` (needs a YAML parser bundled, and tailwind.config is JS —
   QuickJS can eval it). Medium.
3. **`extract`** — run an eval that walks `figma.currentPage` and returns a data tree, then format
   markdown (Rust or bundled). Orchestration-heavy.
4. **`blocks create`** — port `blocks/dashboard-01.js`'s render orchestration (it emits JSX → render).
5. **url-tools** — needs a headless browser or screenshot service; likely stays a wrapper.

Also worth doing:
- CI to cross-compile release binaries (currently only macOS arm64 is in `skill/bin/`).
- Live-test `figjam *` in a FigJam file and `recreate-url`/`screenshot-url`/`remove-bg`.
- A cleaner error when a FigJam-only API is called in a design file.

## Gotchas / sharp edges

- **Two `.git` confusion earlier in the session:** the repo is rooted at `rust/` now. The original
  Node project lives at the parent `figma-cli-main/` (not a git repo from `rust`'s perspective).
- **`undo`** only reverts the last create/render (persisted ids in `~/.figma-ds-cli/last-render.json`).
- **Daemon token** at `~/.figma-ds-cli/.daemon-token`; daemon on `127.0.0.1:3456`; pidfile at
  `~/.figma-cli-daemon.pid`. The Node fallback reuses these, so both CLIs share one daemon.
- **Protocol is load-bearing** — changing `/exec`, `/health`, the WS message types, the token header,
  or the port breaks both the FigCli plugin and the Node fallback (see DECISIONS.md D7).

## Key files to read first

`src/main.rs` (command surface + dispatch) → `src/daemon.rs` (the bridge) →
`src/engine.rs` + `src/tools.rs` (the JS engines) → `src/cmds.rs` (codegen patterns).
The user-facing docs are `rust/README.md` and `rust/skill/SKILL.md`.
