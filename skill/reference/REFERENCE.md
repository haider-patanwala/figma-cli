# figma-cli — command reference

This Rust build implements the Safe-Mode core. All commands talk to a local daemon
that bridges to Figma via the FigCli plugin.

## Connection & daemon

| Command | Description |
|---------|-------------|
| `figma-cli connect` | Start daemon + write/import the FigCli plugin; wait for it to connect. |
| `figma-cli daemon start` | Start the background daemon. |
| `figma-cli daemon status` | Show daemon + plugin connection state. |
| `figma-cli daemon stop` | Stop the daemon. |
| `figma-cli daemon restart` | Restart the daemon. |

The daemon listens on `127.0.0.1:3456`. The plugin connects to it over WebSocket at
`/plugin`. A per-machine token (`~/.figma-ds-cli/.daemon-token`) guards the HTTP API.

## Creating & reading

| Command | Description |
|---------|-------------|
| `figma-cli render '<jsx>'` | Render one JSX frame. `--collection <name>` pins `var:` lookups. |
| `figma-cli render-batch '[…]'` | Render many frames as independent nodes. `--direction row\|col`, `--gap <n>`, `--collection <name>`. |
| `figma-cli tokens preset shadcn` | Create shadcn primitives + semantic (Light/Dark) variable collections. |
| `figma-cli eval '<js>'` | Run raw Plugin API JS (`figma` is global). Reads & mutations only — not for creating visual nodes. |
| `figma-cli find "<query>"` | List nodes whose name contains the query (id, name, type). |
| `figma-cli canvas info` | Current page name, selection count, child count. |
| `figma-cli verify [id]` | Screenshot a node (or selection). `--save <path>` (default `/tmp/figma-verify-<id>.png`), `--measure` for a real-dimension tree. |

## Editing & managing

| Command | Description |
|---------|-------------|
| `figma-cli node to-component <ids…>` | Convert frame(s)/group(s) to components. |
| `figma-cli node delete <ids…>` | Delete node(s) by id. |
| `figma-cli unwrap <id>` | Lift a wrapper's children to its parent, then delete the wrapper. `--keep-wrapper` keeps it. |
| `figma-cli undo` | Remove the node(s) created by the most recent `render` / `render-batch`. |
| `figma-cli var list` | List variable collections, their modes, and variables. |
| `figma-cli var delete-all` | Delete all local variables + collections. `-c <name>` limits to one collection. |
| `figma-cli export <fmt> [id]` | Export to file. `fmt` = png\|svg\|jpg\|pdf; `-o <path>`, `--scale <n>` (raster). |

## Create & edit nodes

| Command | Description |
|---------|-------------|
| `figma-cli create frame/rect/ellipse/text/component/group/autolayout …` | Create elements with smart positioning; hex or `var:` fills/strokes. |
| `figma-cli set fill/stroke/radius/size/scale/pos/opacity/name/text …` | Mutate the selection, `--node <id>`, or `--query <pat>`. |
| `figma-cli sizing hug/fill/fixed`, `padding`, `gap`, `align` | Auto-layout sizing + spacing shortcuts. |
| `figma-cli bind fill/stroke/radius/gap/padding <var>` | Bind a variable to a node property. |
| `figma-cli select/delete/duplicate/get [id]` | Selection + node ops. |
| `figma-cli node-tree [id]` / `node-bindings [id]` | Inspect structure / variable bindings. |

## Variables & tokens

| Command | Description |
|---------|-------------|
| `figma-cli var list/create/find/delete-all` | Manage variables. |
| `figma-cli tokens preset shadcn`, `tokens spacing`, `tokens radii` | Create token collections. |
| `figma-cli export-tokens css/tailwind/dtcg [out]` | Export variables as code. |

## Components & layout

| Command | Description |
|---------|-------------|
| `figma-cli variants from <ids> -p <prop> -v <values>` | Combine into a variant set. |
| `figma-cli prop combine <ids>` | Combine existing components into a variant set. |
| `figma-cli slot create/list/reset/convert` | Slot operations. |

## Quality

| Command | Description |
|---------|-------------|
| `figma-cli a11y contrast/touch/text/audit [id]` | WCAG checks. |
| `figma-cli lint` / `analyze colors/typography/spacing/clusters` | Design analysis. |

## FigJam

`figma-cli figjam sticky/shape/text/connect/move/update/delete/info/nodes` — whiteboard ops.

## Misc

`figma-cli config set/get <key> [value]` — local CLI config.

`--json` is a global flag on every command.

## Architecture (how it works)

```
figma-cli <cmd>  --HTTP-->  daemon (:3456)  --WebSocket /plugin-->  FigCli plugin (in Figma)
                            renders JSX→JS                          evals JS via figma.* API
```

- The JSX→Plugin-API codegen runs in an embedded QuickJS engine inside the daemon
  (bundled from the original tool's proven renderer).
- `<Icon name="prefix:name">` SVGs are fetched from the Iconify API by the daemon;
  if offline, icons fall back to placeholder shapes.

## Gradients & generators (Rust)

| Command | Description |
|---------|-------------|
| `figma-cli gradient extract <image>` | Extract a linear/mesh gradient from a PNG/JPG/WebP/GIF (decoded in Rust). `--mode mesh`, `--apply-to <id>`, `--stops`, `--direction`. |
| `figma-cli gradient mesh "#a,#b,#c"` | Generate a mesh-gradient wallpaper from a palette. `--size`, `--style`, `--apply-to`. |
| `figma-cli shadcn list` / `shadcn add <names> [--all] [--count N]` | shadcn/ui component library (bundled). |

## Node-fallback commands

These heavier/niche commands forward to the bundled Node implementation (set up
by `install.sh` as `js/` next to the binary). They drive the **same** running
daemon + plugin, so behavior is identical to the original tool:

`extract`, `import`, `spec`, `instantiate`, `blocks list|create`, `recreate-url`,
`screenshot-url`, `remove-bg`, `dev`, `section`, `grid`, `annotate`, `plugins`,
`api`, `sizes`, `combos`. Anything else: `figma-cli js <args…>`.

These require Node 18+. The Rust-native commands above do not.
