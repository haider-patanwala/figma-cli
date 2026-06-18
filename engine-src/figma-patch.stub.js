// Stub for ./figma-patch.js used only by the bundled engine.
// The engine never patches Figma (Safe Mode) — it only needs getCdpPort to
// exist because figma-client.js imports it at module top-level.
export function getCdpPort() {
  return 9222;
}
export default { getCdpPort };
