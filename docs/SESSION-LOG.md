# Session log

Chronological record of how the Rust port was built, mapped to commits. Newest first
within phases. (Commit hashes are from the `rust/` repo; an external commit `6eb1c92`
by the user refactored package.json/manifest.json mid-stream.)

## Phase 1 — core vertical slice (`dcb0a29` initial commit)
Proved the whole architecture end-to-end:
- clap CLI + `--json`, transport client, daemon lifecycle.
- Daemon: axum HTTP `/exec` + `/health`, WS `/plugin` bridge, pending-request map,
  hello/ping/pong, 25s/60s timeouts, token auth — byte-compatible with the JS daemon.
- Render engine: esbuild-bundled `parseJSX`/`parseJSXBatch` in QuickJS; Rust-side Iconify fetch.
- Commands: connect, daemon, eval, render, render-batch, tokens preset shadcn, find, canvas info, verify.
- Verified with `test-helpers/fake-plugin.mjs` (offline WS round-trip) before any Figma.

## Phase 2 — node ops, undo, vars, export (`45c80a7`)
node to-component/delete, unwrap, undo (persisted ids), var list/delete-all, export png/svg/jpg/pdf.

## Phase 3 — full parity, in batches
- **batch 1** (`8cd85b1`): create frame/rect/ellipse/text/component/group/autolayout; set
  fill/stroke/radius/size/scale/pos/opacity/name/text; sizing/padding/gap/align; bind ×5;
  select/delete/duplicate/get. Added `jsgen.rs` + `cmds.rs`.
- **batch 2** (`5e21a3a`): a11y contrast/touch/text/audit (asset payloads via `run_asset`),
  node-tree/node-bindings, lint, analyze colors/typography/spacing/clusters.
- **batch 3** (`d4d20bc`): slot create/list/reset/convert, variants from, prop combine.
- **batch 4** (`9a6138d`): tokens spacing/radii, export-tokens css/tailwind/dtcg, var create/find,
  config set/get (local file).
- **batch 5** (`0bb2fcb`): figjam sticky/shape/text/connect/move/update/delete/info/nodes.
- **batch 6** (`e5d401c`): var visualize + create-batch/bind-batch/set-batch/rename-batch, delete-batch.
- **batch 7** (`7821e5a`): gradient extract (Rust `image` decode → tools engine analysis) + mesh.
  Introduced the second QuickJS engine (`tools.rs`, `assets/tools.js`).
- **batch 8** (`e4e2333`): shadcn list/add (shadcn.js bundled into tools engine → render-batch).
- **batch 9** (`2375f0d`): Node fallback (`passthrough.rs`) for extract/import/spec/instantiate/
  blocks/recreate-url/screenshot-url/remove-bg/dev/section/grid/annotate/plugins/api/sizes/combos
  + `js` escape hatch. Completed parity (~57 commands).

## Packaging & docs
- `fe88c50`: rust/README.md.
- `707af09`: moved the skill package into `rust/skill` (was at repo-root `skill/`).
- `b16ddcb`: decoupled `rust/` from the repo root — build scripts read vendored `skill/js/src`;
  passthrough dev path inside the crate. Verified build+run with the repo root removed.
- (this commit): added `rust/docs/`.

## Live verification against real Figma (this session)
With Figma open + FigCli connected (`daemon status` → `"plugin": true`):
- eval, canvas info (Page 1, editor=figma) ✓
- render button + Iconify rocket icon ✓ (screenshot looked correct)
- render-batch (3 independent cards) ✓; verify (PNG + measure tree) ✓
- tokens preset shadcn (244+32) ✓; var list ✓; `var:primary` binding ✓ (near-black themed button)
- shadcn add button (variant gallery) ✓; shadcn list ✓
- gradient mesh wallpaper ✓ (smooth multi-color blobs); gradient extract on a real PNG ✓
- set fill/name, bind fill, create rect/ellipse/text, node to-component, get, duplicate, undo ✓
- a11y contrast (7 fails) / touch (26 fails), analyze colors, lint, node-tree ✓
- export-tokens css/dtcg, var visualize (244 swatches) ✓
- **Node fallback** (each drove the same Rust daemon): `extract` read 447 nodes → DESIGN.md ✓;
  `import` ✓; `spec` ✓; `api setup` + `api Frame` ✓; **`blocks create dashboard-01`** ✓
  (full dashboard rendered correctly).

### Known limitations / not verified live
- `figjam *` needs a FigJam file open (`createSticky` only exists there); errored correctly in a
  design file.
- `recreate-url` / `screenshot-url` / `remove-bg` not exercised (browser/remove.bg API);
  the fallback *mechanism* is proven by extract/import/blocks/api working.
- Bundle regeneration (`build-engine.sh`/`build-tools.sh`) needs the `esbuild` tool present.
- Test nodes from the live run were left on the user's canvas (not auto-deleted, per the
  never-delete rule).
