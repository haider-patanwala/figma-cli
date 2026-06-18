// Engine entry: exposes the proven JSX->Plugin-API codegen from
// figma-client.js to the Rust host (via globalThis.__engine), with the only
// impure dependency — Iconify icon fetching — replaced by a Rust-supplied map.
//
// The Rust daemon: (1) calls __engine.iconNames(jsxArrayJson) to learn which
// icons a render needs, (2) fetches those SVGs itself (blocking reqwest),
// (3) calls __engine.parseOne / parseBatch passing the fetched icon map.
//
// All arguments and results cross the FFI boundary as JSON strings to keep the
// QuickJS<->Rust marshalling trivial.

import { FigmaClient } from "./figma-client.js";

function clientWith(iconMap, collection) {
  const c = new FigmaClient();
  if (collection) c.setCollection(collection);
  c.prefetchIconSvgs = function () {
    return iconMap || {};
  };
  return c;
}

// Collect every icon name referenced across the given JSX strings.
// In: JSON array of JSX strings. Out: JSON array of "prefix:name" strings.
function iconNames(jsxArrayJson) {
  const jsxArray = JSON.parse(jsxArrayJson);
  const c = new FigmaClient();
  const names = new Set();
  for (const jsx of jsxArray) {
    const m = jsx.match(/<Frame\s+([^>]*)>/);
    if (!m) continue;
    const start = m.index + m[0].length;
    const children = c.extractContent(jsx.slice(start), "Frame");
    const els = c.parseChildren(children);
    for (const n of c.collectIconNames(els)) names.add(n);
  }
  return JSON.stringify([...names]);
}

// Single-frame codegen (mirrors parseJSX without the async icon prefetch).
// Returns the JS payload string the plugin will eval.
function parseOne(jsx, iconMapJson, collection) {
  const iconMap = iconMapJson ? JSON.parse(iconMapJson) : {};
  const c = clientWith(iconMap, collection || null);
  const openMatch = jsx.match(/<Frame\s+([^>]*)>/);
  if (!openMatch) throw new Error("Invalid JSX: must start with <Frame>");
  const startIdx = openMatch.index + openMatch[0].length;
  const children = c.extractContent(jsx.slice(startIdx), "Frame");
  const props = c.parseProps(openMatch[1]);
  const childElements = c.parseChildren(children);
  return c.generateCode(props, childElements, iconMap);
}

// Batch codegen. parseJSXBatch is async only because of prefetchIconSvgs,
// which we made synchronous, so the returned promise resolves immediately;
// the Rust host drives it to completion via Promise::finish.
function parseBatch(jsxArrayJson, iconMapJson, gap, vertical, collection) {
  const jsxArray = JSON.parse(jsxArrayJson);
  const iconMap = iconMapJson ? JSON.parse(iconMapJson) : {};
  const c = clientWith(iconMap, collection || null);
  return c.parseJSXBatch(jsxArray, { gap: gap, vertical: vertical });
}

globalThis.__engine = { iconNames, parseOne, parseBatch };
