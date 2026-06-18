// Touch target sizes (WCAG 2.5.8). Params: __args.nodeId, __args.minSize.
// Ported from src/commands/a11y.js `touch`.
const targetId = __args.nodeId || null;
const root = targetId ? await figma.getNodeByIdAsync(targetId) : figma.currentPage;
if (!root) return { error: 'Node not found' };
const minSize = __args.minSize || 44;
const results = [];
const interactivePatterns = /button|btn|link|tab|toggle|switch|checkbox|radio|input|select|dropdown|menu|icon-btn|close|nav|click|tap|cta/i;
function traverse(node) {
  if (node.visible === false) return;
  const isInteractive = (node.type === 'INSTANCE' || node.type === 'COMPONENT' || interactivePatterns.test(node.name) || (node.reactions && node.reactions.length > 0));
  if (isInteractive) {
    const w = Math.round(node.width), h = Math.round(node.height);
    const pass = w >= minSize && h >= minSize;
    const wcag248 = w >= 24 && h >= 24;
    results.push({ id: node.id, name: node.name, type: node.type, width: w, height: h, pass, wcag248,
      issue: !pass ? (w < minSize && h < minSize ? 'both' : w < minSize ? 'width' : 'height') : null });
  }
  if ('children' in node) for (const child of node.children) traverse(child);
}
if ('children' in root) for (const child of root.children) traverse(child);
const passing = results.filter(r => r.pass);
const failing = results.filter(r => !r.pass);
const critical = results.filter(r => !r.wcag248);
return { minSize, total: results.length, passing: passing.length, failing: failing.length, critical: critical.length, issues: failing };
