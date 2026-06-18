// Stub for the `ws` package. The bundled engine only does pure JSX->JS
// codegen; the WebSocket transport lives in the Rust daemon, so this class is
// never instantiated. It exists solely to satisfy the top-level import in
// figma-client.js.
export default class WebSocket {}
