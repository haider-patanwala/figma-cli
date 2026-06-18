// Tools engine: exposes the proven pure-JS modules (gradient analysis, and
// later design-md / code-import / renderers) to the Rust host via
// globalThis.__tools. Image decoding is done in Rust and injected as
// opts.__img = { width, height, data: [r,g,b,a, ...] }.

import {
  extractGradient, extractMesh, buildMeshFromColors, buildFigmaPaint, buildCssString,
} from "./gradient-extractor.js";

globalThis.__tools = {
  // Linear gradient from injected pixels. argsJson: { img, direction, stops, trim }
  gradientExtract(argsJson) {
    const a = JSON.parse(argsJson);
    const result = extractGradient("__INJECTED__", { __img: a.img, direction: a.direction, stops: a.stops, trim: a.trim });
    return JSON.stringify({ result, css: buildCssString(result), paint: buildFigmaPaint(result) });
  },
  // Mesh recipe from injected pixels. argsJson: { img, trim, blur }
  meshExtract(argsJson) {
    const a = JSON.parse(argsJson);
    const recipe = extractMesh("__INJECTED__", { __img: a.img, trim: a.trim });
    if (a.blur != null) recipe.blurFraction = a.blur;
    return JSON.stringify(recipe);
  },
  // Mesh recipe from a color palette (no image). argsJson: { colors, base, blur, style, seed }
  meshFromColors(argsJson) {
    const a = JSON.parse(argsJson);
    const recipe = buildMeshFromColors(a.colors, { base: a.base, blur: a.blur, style: a.style, seed: a.seed });
    return JSON.stringify(recipe);
  },
};
