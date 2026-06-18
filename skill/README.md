# figma-cli skill

An agent skill that controls **Figma Desktop** from the command line via the
`figma-cli` Rust binary and the **FigCli** plugin bridge — no API key, no cloud
(Safe Mode). Designed for publishing to [skills.sh](https://skills.sh).

## Contents

```
SKILL.md             Agent instructions (loaded by the agent)
reference/
  REFERENCE.md       Full command reference
  JSX.md             JSX render syntax + pitfalls
bin/
  install.sh         Installs figma-cli (+ Node fallback) onto PATH
  figma-cli-<triple> Prebuilt binary (per platform; built via cargo)
plugin/
  manifest.json      FigCli Figma plugin (imported into Figma Desktop)
  code.js
  ui.html
js/                  Node fallback CLI (drives the same daemon; needs Node 18+)
  index.js, src/, package.json
```

## How it works

`figma-cli` runs a local daemon on `127.0.0.1:3456`. The FigCli plugin, running
inside Figma Desktop, connects to the daemon over a WebSocket and executes the
Figma Plugin API. The CLI sends JSX (compiled to Plugin-API JS by an embedded
QuickJS engine) or raw JS; node creation happens *inside Figma*.

```
figma-cli <cmd> --HTTP--> daemon (:3456) --WS /plugin--> FigCli plugin (in Figma)
```

No Figma application files are modified (this is the plugin-based "Safe Mode";
the binary never patches Figma's app bundle).

**Node fallback:** most commands are pure Rust, but a few heavy/niche ones
(`extract`, `import`, `spec`, `instantiate`, `blocks`, `recreate-url`,
`screenshot-url`, `remove-bg`, `dev`, `section`, `grid`, `annotate`, `plugins`,
`api`, `sizes`, `combos`) forward to the bundled `js/` CLI, which speaks the same
daemon protocol and drives the same daemon + plugin. `install.sh` places `js/`
next to the binary and runs `npm install` so the binary auto-locates it. These
need Node 18+; the Rust-native commands do not.

## Install (for users)

```bash
bash bin/install.sh        # installs figma-cli to ~/.local/bin
figma-cli connect          # start daemon + import the FigCli plugin
```

## Building binaries (for maintainers)

The source lives in the parent `rust/` crate. To produce a release binary for the
host platform:

```bash
cd ..                       # the rust/ crate root
cargo build --release
cp target/release/figma-cli skill/bin/figma-cli-$(rustc -vV | awk '/host/{print $2}')
```

Cross-compile for other targets (e.g. via `cross` or CI) and drop the resulting
`figma-cli-<target-triple>` files into `bin/`; `install.sh` picks the right one.

## Publishing to skills.sh

This directory is the skill package. Publish it per skills.sh's submission flow
(account + CLI/web upload). Binaries are bundled in `bin/`; keep them current with
each release. The `plugin/` assets must ship with the skill so users can import
FigCli into Figma.
