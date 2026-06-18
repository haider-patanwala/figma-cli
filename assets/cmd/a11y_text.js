// Text accessibility (sizes, line height, spacing). Params: __args.nodeId.
// Ported from src/commands/a11y.js `text`.
const targetId = __args.nodeId || null;
const root = targetId ? await figma.getNodeByIdAsync(targetId) : figma.currentPage;
if (!root) return { error: 'Node not found' };
const results = [];
function traverse(node) {
  if (node.visible === false) return;
  if (node.type === 'TEXT') {
    const fontSize = typeof node.fontSize === 'number' ? node.fontSize : null;
    const lineHeight = node.lineHeight;
    let lineHeightValue = null, lineHeightRatio = null;
    if (lineHeight && lineHeight.unit === 'PIXELS') { lineHeightValue = lineHeight.value; if (fontSize) lineHeightRatio = lineHeight.value / fontSize; }
    else if (lineHeight && lineHeight.unit === 'PERCENT') { lineHeightRatio = lineHeight.value / 100; if (fontSize) lineHeightValue = fontSize * lineHeightRatio; }
    const issues = [];
    if (fontSize && fontSize < 12) issues.push({ rule: 'min-size', message: 'Font size < 12px (hard to read)', severity: 'error' });
    else if (fontSize && fontSize < 14) issues.push({ rule: 'min-size', message: 'Font size < 14px (consider increasing for body text)', severity: 'warning' });
    if (fontSize && fontSize <= 18 && lineHeightRatio && lineHeightRatio < 1.5) issues.push({ rule: 'line-height', message: 'Line height < 1.5x for body text (WCAG 1.4.12)', severity: 'warning' });
    if (node.paragraphSpacing !== undefined && fontSize && node.paragraphSpacing > 0 && node.paragraphSpacing < fontSize * 2) issues.push({ rule: 'paragraph-spacing', message: 'Paragraph spacing < 2x font size (WCAG 1.4.12)', severity: 'warning' });
    if (node.letterSpacing && node.letterSpacing.unit === 'PIXELS' && fontSize && node.letterSpacing.value < fontSize * 0.12 && node.letterSpacing.value !== 0) issues.push({ rule: 'letter-spacing', message: 'Letter spacing < 0.12x font size (WCAG 1.4.12)', severity: 'warning' });
    if (node.textCase === 'UPPER' && node.characters && node.characters.length > 20) issues.push({ rule: 'all-caps', message: 'Long ALL CAPS text (> 20 chars) reduces readability', severity: 'warning' });
    results.push({ id: node.id, name: node.name, text: node.characters ? node.characters.substring(0, 40) : '', fontSize,
      lineHeight: lineHeightValue ? Math.round(lineHeightValue * 10) / 10 : null,
      lineHeightRatio: lineHeightRatio ? Math.round(lineHeightRatio * 100) / 100 : null, issues });
  }
  if ('children' in node) for (const child of node.children) traverse(child);
}
if ('children' in root) for (const child of root.children) traverse(child);
const withIssues = results.filter(r => r.issues.length > 0);
const errors = withIssues.filter(r => r.issues.some(i => i.severity === 'error'));
const warnings = withIssues.filter(r => r.issues.some(i => i.severity === 'warning') && !r.issues.some(i => i.severity === 'error'));
return { total: results.length, errors: errors.length, warnings: warnings.length, passing: results.length - withIssues.length, issues: withIssues };
