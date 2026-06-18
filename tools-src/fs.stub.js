// stub: fs not available in QuickJS; host injects pixels.
export function readFileSync() { throw new Error("fs unavailable in tools engine"); }
