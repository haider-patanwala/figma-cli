export function extname(p){ const m = String(p).match(/\.[^.\/]+$/); return m ? m[0] : ''; }
export default { extname };
