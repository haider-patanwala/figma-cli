// Export local variables as W3C Design Tokens (DTCG) JSON. No params.
const vars = await figma.variables.getLocalVariablesAsync();
const byId = {};
for (const v of vars) byId[v.id] = v.name;
const dot = n => n.replace(/\//g, '.');
const h2 = n => Math.round(n*255).toString(16).padStart(2,'0');
const toColor = c => { const b = '#'+h2(c.r)+h2(c.g)+h2(c.b); return (c.a != null && c.a < 1) ? b+h2(c.a) : b; };
const tree = {};
const setPath = (path, token) => { const p = path.split('/'); let cur = tree; for (let i=0;i<p.length-1;i++){ if (!cur[p[i]] || cur[p[i]].$value !== undefined) cur[p[i]] = {}; cur = cur[p[i]]; } cur[p[p.length-1]] = token; };
for (const v of vars) {
  const val = Object.values(v.valuesByMode)[0];
  const dtype = v.resolvedType === 'COLOR' ? 'color' : v.resolvedType === 'FLOAT' ? 'dimension' : v.resolvedType === 'BOOLEAN' ? 'boolean' : 'string';
  let token;
  if (val && val.type === 'VARIABLE_ALIAS') { const ref = byId[val.id]; token = { $type: dtype, $value: ref ? '{'+dot(ref)+'}' : null }; }
  else if (v.resolvedType === 'COLOR') token = { $type: 'color', $value: toColor(val) };
  else if (v.resolvedType === 'FLOAT') token = { $type: 'dimension', $value: val + 'px' };
  else if (v.resolvedType === 'BOOLEAN') token = { $type: 'boolean', $value: val };
  else token = { $type: 'string', $value: String(val) };
  if (v.description) token.$description = v.description;
  setPath(v.name, token);
}
return JSON.stringify(tree, null, 2);
