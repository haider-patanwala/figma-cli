// var set-batch — params: __args.operations (array), __args.colFilter (string|null).
const operations = __args.operations;
const colFilter = __args.colFilter;

function hexToRgb(hex) {
  const result = /^#?([a-f\\d]{2})([a-f\\d]{2})([a-f\\d]{2})$/i.exec(hex);
  return result ? { r: parseInt(result[1], 16) / 255, g: parseInt(result[2], 16) / 255, b: parseInt(result[3], 16) / 255 } : null;
}

// Load the variable map once, with the same scoping rules as render.
const [allCols, allVars] = await Promise.all([
  figma.variables.getLocalVariableCollectionsAsync(),
  figma.variables.getLocalVariablesAsync(),
]);
let scoped = null;
if (colFilter) {
  const fl = colFilter.toLowerCase();
  const cols = allCols.filter(c => c.name.toLowerCase() === fl || c.name.toLowerCase().includes(fl));
  scoped = new Set(cols.map(c => c.id));
}
const shadcnIds = new Set(allCols.filter(c => c.name.startsWith('shadcn')).map(c => c.id));
const varCache = {};
const register = (v) => {
  if (!varCache[v.name]) varCache[v.name] = v;
  const slash = v.name.lastIndexOf('/');
  if (slash >= 0) {
    const tail = v.name.slice(slash + 1);
    if (tail && !varCache[tail]) varCache[tail] = v;
  }
};
const qualified = {};
for (const v of allVars) {
  const col = allCols.find(c => c.id === v.variableCollectionId);
  if (!col) continue;
  qualified[col.name.toLowerCase() + ':' + v.name] = v;
  const slash = v.name.lastIndexOf('/');
  if (slash >= 0) qualified[col.name.toLowerCase() + ':' + v.name.slice(slash + 1)] = v;
}
if (scoped) {
  for (const v of allVars) if (scoped.has(v.variableCollectionId)) register(v);
} else {
  for (const v of allVars) if (shadcnIds.has(v.variableCollectionId)) register(v);
  for (const v of allVars) if (!shadcnIds.has(v.variableCollectionId)) register(v);
}
const lookupVar = (ref) => {
  // Accept "primary", "colors/primary", "miro:primary" — return Variable or null
  if (ref.includes(':')) {
    const [cn, vn] = ref.split(':', 2);
    return qualified[cn.toLowerCase() + ':' + vn] || varCache[vn] || null;
  }
  return varCache[ref] || null;
};
const setPaintColor = (input) => {
  // Returns a Paint with a SOLID color, either hex (frozen) or variable-bound.
  if (typeof input === 'string' && input.startsWith('var:')) {
    const ref = input.slice(4);
    const v = lookupVar(ref);
    if (!v) return { _err: 'variable not found: ' + ref };
    return figma.variables.setBoundVariableForPaint(
      { type: 'SOLID', color: { r: 0.5, g: 0.5, b: 0.5 } }, 'color', v
    );
  }
  const rgb = hexToRgb(input);
  return rgb ? { type: 'SOLID', color: rgb } : { _err: 'invalid color: ' + input };
};

let updated = 0;
const notFound = [];
const errors = [];

for (const op of operations) {
  const node = await figma.getNodeByIdAsync(op.nodeId);
  if (!node) { notFound.push(op.nodeId); continue; }
  let touched = false;

  if (op.fill !== undefined && 'fills' in node) {
    const paint = setPaintColor(op.fill);
    if (paint._err) errors.push(op.nodeId + ': ' + paint._err);
    else { node.fills = [paint]; touched = true; }
  }
  if (op.stroke !== undefined && 'strokes' in node) {
    const paint = setPaintColor(op.stroke);
    if (paint._err) errors.push(op.nodeId + ': ' + paint._err);
    else { node.strokes = [paint]; touched = true; }
  }
  if (op.strokeWidth !== undefined && 'strokeWeight' in node) { node.strokeWeight = op.strokeWidth; touched = true; }
  if (op.radius !== undefined && 'cornerRadius' in node) { node.cornerRadius = op.radius; touched = true; }
  if (op.opacity !== undefined && 'opacity' in node) { node.opacity = op.opacity; touched = true; }
  if (op.name && 'name' in node) { node.name = op.name; touched = true; }
  if (op.visible !== undefined && 'visible' in node) { node.visible = op.visible; touched = true; }
  if (op.x !== undefined) { node.x = op.x; touched = true; }
  if (op.y !== undefined) { node.y = op.y; touched = true; }
  if (op.width !== undefined && op.height !== undefined && 'resize' in node) {
    node.resize(op.width, op.height); touched = true;
  }
  if (touched) updated++;
}
return { updated, notFound, errors };
