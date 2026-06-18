// stub: pixels are decoded+injected by the Rust host; JS decode is never used.
export const PNG = { sync: { read() { throw new Error("decode in host"); } } };
export default { PNG };
