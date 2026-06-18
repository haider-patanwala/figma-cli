---
name: figma-cli
description: Control Figma Desktop directly from the command line — create frames, components, design tokens/variables, gradients, accessibility audits and more, with no API key — via the figma-cli Rust binary and the FigCli plugin bridge (Safe Mode). Use when the user wants to build, render, restyle, audit, export, or extract designs in Figma, manage variables/tokens, run shadcn/ui components, or connect an agent to their Figma desktop app.
---

# figma-cli

Drives **Figma Desktop** over a local plugin bridge. No API key, no cloud, no MCP.
A single Rust binary runs a daemon; the bundled **FigCli** plugin (running inside
Figma) connects to it and executes the Figma Plugin API. You generate designs by
sending JSX or raw Plugin-API JS — it runs *inside Figma*.

**Run commands silently and give the user friendly, plain-language feedback. Never
paste raw terminal commands at them.**

## Setup (once)

1. Install: `bash bin/install.sh` (puts `figma-cli` on PATH; also sets up the Node
   fallback used by a few heavy commands — needs Node 18+).
2. Open a **design file** in Figma Desktop.
3. Run `figma-cli connect`. It starts the daemon and prints how to import the FigCli
   plugin: **Plugins → Development → Import plugin from manifest…** → pick the printed
   `manifest.json` → run **FigCli** and keep its small window open.
4. Re-run `figma-cli connect` — it should report **Connected**.

Check state anytime with `figma-cli daemon status` (`"plugin": true` = ready).
Add `--json` to any command for machine-readable output.

## Intent → command

| User says | Command |
|-----------|---------|
| "connect to figma" | `figma-cli connect` |
| "create a frame / button / card" | `figma-cli render '<Frame …>…</Frame>'` |
| "create N buttons/cards" (independent) | `figma-cli render-batch '["<Frame…>", …]' --direction row` |
| "create 3 buttons" (shadcn primitives) | `figma-cli shadcn add button --count 3` |
| "list shadcn components" | `figma-cli shadcn list` |
| "add shadcn colors / tokens" | `figma-cli tokens preset shadcn` |
| "add a spacing / radius scale" | `figma-cli tokens spacing` / `figma-cli tokens radii` |
| "show colors on canvas" | `figma-cli var visualize` |
| "list / find variables" | `figma-cli var list` / `figma-cli var find <q>` |
| "create a variable" | `figma-cli var create <name> -c <collection> -t COLOR -v "#fff"` |
| "delete all variables" | `figma-cli var delete-all` |
| "export tokens as css/tailwind/dtcg" | `figma-cli export-tokens css` (or `tailwind` / `dtcg out.json`) |
| "set fill / rename / resize a node" | `figma-cli set fill "#fff" --node <id>` (also `name/size/pos/opacity/radius/scale/text`) |
| "bind a variable to fill/stroke/radius/gap/padding" | `figma-cli bind fill <var> --node <id>` |
| "make it hug / fill / fixed" | `figma-cli sizing hug` (or `fill` / `fixed <w> [h]`) |
| "convert to component" | `figma-cli node to-component <id>` |
| "combine into variants" | `figma-cli variants from <ids> -p Size -v Small,Medium,Large` |
| "create a slot" | `figma-cli slot create "Content"` |
| "undo that / remove what you just made" | `figma-cli undo` |
| "you bundled them — unwrap" | `figma-cli unwrap <wrapperId>` |
| "select / duplicate / delete / inspect" | `figma-cli select\|duplicate\|delete\|get <id>` |
| "what's on the canvas" | `figma-cli canvas info` / `figma-cli node-tree` |
| "screenshot / verify that node" | `figma-cli verify [id] --save out.png` (`--measure` for sizes) |
| "export as png/svg/jpg/pdf" | `figma-cli export png <id> -o out.png` |
| "check contrast / touch targets / text" | `figma-cli a11y contrast` (or `touch` / `text` / `audit`) |
| "lint / analyze colors-typography-spacing" | `figma-cli lint` / `figma-cli analyze colors` |
| "extract a gradient from this image" | `figma-cli gradient extract <image> --apply-to <id>` |
| "make a mesh wallpaper from these colors" | `figma-cli gradient mesh "#a,#b,#c" --size 1920x1080` |
| "export the design system as markdown" | `figma-cli extract DESIGN.md` |
| "import tokens / tailwind.config / css" | `figma-cli import <file>` |
| "what component exists for X / its spec" | `figma-cli spec <name>` |
| "use the existing X component" | `figma-cli instantiate <name>` |
| "create a dashboard" | `figma-cli blocks create dashboard-01` |
| "recreate / screenshot this URL" | `figma-cli recreate-url <url>` / `figma-cli screenshot-url <url>` |
| "run raw plugin-api code" | `figma-cli eval '<js>'` |

Full reference: [reference/REFERENCE.md](reference/REFERENCE.md). JSX syntax + pitfalls:
[reference/JSX.md](reference/JSX.md).

## Rules

1. **Use `render` for frames** — it has smart positioning. Never build visual nodes
   with raw `eval` (overlaps at 0,0, bypasses guards). `eval` is for reads and for
   mutating *existing* nodes only.
2. **N items the user asked for = N independent top-level nodes.** Use `render-batch`
   (or `shadcn add --count N`), never one wrapper Frame/Component containing N children.
   If you bundled them by accident, `figma-cli unwrap <id>` rescues the children.
3. **Never delete nodes the user already has** — they may be keeping them.
4. **Always `verify --save <path>`** for visual checks — it writes the PNG to disk and
   returns dimensions, keeping context lean (don't dump base64). Use `--measure` to
   catch size bugs by numbers, not by eyeballing.
5. **After creating, `verify`** to confirm it looks right.
6. **Themed vs shadcn:** if the user says "using variables / themed / with tokens",
   render var-bound frames (`bg="var:primary"`), don't fall back to shadcn's own colors.
   Pin a specific collection with `--collection <name>` when several are loaded.

## JSX essentials

```jsx
<Frame bg="#3b82f6" px={16} py={10} rounded={10} flex="row" justify="center" items="center">
  <Text color="#fff" size={14} weight="medium">Button</Text>
  <Icon name="lucide:arrow-right" size={16} color="#fff" />
</Frame>
```

- `flex="row"|"col"`, `gap`, `p`/`px`/`py`, `justify`, `items`, `w`/`h` (`"fill"`/`"hug"`).
- `bg`/`stroke` accept `var:name` to bind to a variable (`bg="var:primary"`).
- **Text wrapping (most common bug):** set `w="fill"` on the parent **and** every `<Text>`.
- No emojis — use `<Icon name="lucide:…">` (real SVG) or shapes.

## Architecture & fallback note

Node creation runs *inside Figma* via the plugin. JSX→Plugin-API codegen, gradient
analysis, and the shadcn library run in embedded QuickJS engines in the binary;
images and icons are fetched/decoded in Rust. A few heavy/niche commands (`extract`,
`import`, `spec`, `instantiate`, `blocks`, `recreate-url`, `screenshot-url`,
`remove-bg`, `dev`, `section`, `grid`, `annotate`, `plugins`, `api`, `sizes`,
`combos`) forward to the bundled Node CLI, which drives the **same** daemon + plugin
(needs Node 18+). Everything else is pure Rust.
