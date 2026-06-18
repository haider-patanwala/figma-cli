// Test helper: simulates the FigCli plugin's WebSocket protocol so the daemon
// and codegen can be exercised WITHOUT opening Figma. It connects to the daemon,
// logs the JS payloads it receives, and replies with canned results.
//
// Usage:
//   1. figma-cli daemon start
//   2. node rust/test-helpers/fake-plugin.mjs   (leave running)
//   3. figma-cli render '<Frame ...>...</Frame>'   etc.
//
// Requires the `ws` package (already in the parent repo's node_modules).
import WebSocket from "../../node_modules/ws/index.js";

const ws = new WebSocket("ws://127.0.0.1:3456/plugin");

ws.on("open", () => {
  ws.send(JSON.stringify({ type: "hello", mode: "plugin", version: "fake-1.0" }));
  console.error("[fake-plugin] connected");
});

ws.on("message", (data) => {
  const msg = JSON.parse(data.toString());
  if (msg.action === "eval") {
    console.error(`[fake-plugin] eval id=${msg.id} bytes=${msg.code.length}`);
    console.error("  head:", JSON.stringify(msg.code.slice(0, 100)));
    // Pretend Figma created a node.
    ws.send(JSON.stringify({ type: "result", id: msg.id, result: { id: "1:23", name: "created" } }));
  }
  if (msg.action === "eval-batch") {
    console.error(`[fake-plugin] eval-batch id=${msg.id} count=${msg.codes.length}`);
    const results = msg.codes.map((_, i) => ({ success: true, result: "1:" + i }));
    ws.send(JSON.stringify({ type: "batch-result", id: msg.id, results }));
  }
  if (msg.action === "ping") ws.send(JSON.stringify({ type: "pong", id: msg.id }));
});

ws.on("close", () => console.error("[fake-plugin] closed"));
ws.on("error", (e) => console.error("[fake-plugin] error:", e.message));
