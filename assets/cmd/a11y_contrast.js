// WCAG contrast check. Params: __args.nodeId (string|null), __args.level ('AA'|'AAA').
// Body is wrapped by the host in an async IIFE with __args defined. Ported from
// src/commands/a11y.js `contrast`.
const targetId = __args.nodeId || null;
const root = targetId ? await figma.getNodeByIdAsync(targetId) : figma.currentPage;
if (!root) return { error: 'Node not found' };

function luminance(r, g, b) {
  const [rs, gs, bs] = [r, g, b].map(c => c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}
function contrastRatio(l1, l2) {
  const lighter = Math.max(l1, l2), darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}
function getSolidColor(node) {
  if (node.fills && Array.isArray(node.fills)) {
    for (const fill of node.fills) {
      if (fill.type === 'SOLID' && fill.visible !== false) {
        const o = fill.opacity !== undefined ? fill.opacity : 1;
        return { r: fill.color.r, g: fill.color.g, b: fill.color.b, a: o };
      }
    }
  }
  return null;
}
function getBgColor(node) {
  let current = node.parent;
  while (current) {
    const color = getSolidColor(current);
    if (color && color.a > 0.01) return color;
    current = current.parent;
  }
  return { r: 1, g: 1, b: 1, a: 1 };
}
function blendOnWhite(fg, bg) {
  const a = fg.a;
  return { r: fg.r * a + bg.r * (1 - a), g: fg.g * a + bg.g * (1 - a), b: fg.b * a + bg.b * (1 - a) };
}
function toHex(c) {
  const r = Math.round(c.r * 255).toString(16).padStart(2, '0');
  const g = Math.round(c.g * 255).toString(16).padStart(2, '0');
  const b = Math.round(c.b * 255).toString(16).padStart(2, '0');
  return '#' + r + g + b;
}
const results = [];
function traverse(node) {
  if (node.type === 'TEXT' && node.visible !== false) {
    const textColor = getSolidColor(node);
    if (!textColor) return;
    const bgColor = getBgColor(node);
    const fg = blendOnWhite(textColor, { r: 1, g: 1, b: 1 });
    const bg = blendOnWhite(bgColor, { r: 1, g: 1, b: 1 });
    const ratio = contrastRatio(luminance(fg.r, fg.g, fg.b), luminance(bg.r, bg.g, bg.b));
    const fontSize = typeof node.fontSize === 'number' ? node.fontSize : 16;
    const fontWeight = node.fontWeight || 400;
    const isLarge = fontSize >= 18 || (fontSize >= 14 && fontWeight >= 700);
    const aaPass = isLarge ? ratio >= 3 : ratio >= 4.5;
    const aaaPass = isLarge ? ratio >= 4.5 : ratio >= 7;
    results.push({ id: node.id, name: node.name, text: node.characters ? node.characters.substring(0, 50) : '',
      fontSize, isLarge, fgColor: toHex(fg), bgColor: toHex(bg), ratio: Math.round(ratio * 100) / 100, aa: aaPass, aaa: aaaPass });
  }
  if ('children' in node) for (const child of node.children) { if (child.visible !== false) traverse(child); }
}
if ('children' in root) { for (const child of root.children) traverse(child); } else { traverse(root); }
const level = __args.level || 'AA';
const passing = results.filter(r => level === 'AAA' ? r.aaa : r.aa);
const failing = results.filter(r => level === 'AAA' ? !r.aaa : !r.aa);
return { level, total: results.length, passing: passing.length, failing: failing.length, issues: failing };
