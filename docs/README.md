# rust/docs — session memory & handoff

These docs let another agent (or a future you) resume work on the Rust port of
`figma-cli` without re-deriving context. Read them in this order:

1. **[RESUMING.md](RESUMING.md)** — start here. Current state, how to build/run/test,
   how to add a command, what's done vs. open.
2. **[ARCHITECTURE.md](ARCHITECTURE.md)** — how the system works: daemon, plugin
   bridge, the two QuickJS engines, the Node fallback, the wire protocol, file map.
3. **[DECISIONS.md](DECISIONS.md)** — the key decisions and *why* (Safe Mode only,
   bundle-don't-rewrite, QuickJS, Node fallback, repo layout, self-containment).
4. **[SESSION-LOG.md](SESSION-LOG.md)** — chronological record of what was built and
   tested, mapped to commits.

**One-line summary:** a single Rust binary that drives Figma Desktop via the FigCli
plugin (Safe Mode, no API key). ~57 commands at full parity with the original Node
`figma-ds-cli` — most reimplemented natively in Rust, the heavy/niche ones forwarded
to a bundled Node CLI that drives the same daemon. Tested live against real Figma.

User intent that shaped this work (verbatim constraints):
- Rewrite figma-cli in Rust as a binary; ship it as an installable skill for skills.sh.
- **Safe / plugin approach only** — never patch the Figma app.
- **Bundle the JS engine, port the shell** — reuse proven codegen, don't re-derive.
- Port **all** commands; use Rust where possible, else a wrapper.
- The `rust/` dir must be self-contained (no repo-root dependency to build/run).
