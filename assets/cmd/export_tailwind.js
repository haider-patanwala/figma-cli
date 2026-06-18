// Export color variables as a Tailwind theme config. No params.
const vars = await figma.variables.getLocalVariablesAsync();
const colorVars = vars.filter(v => v.resolvedType === 'COLOR');
const colors = {};
colorVars.forEach(v => {
  const val = Object.values(v.valuesByMode)[0];
  const hex = '#' + [val.r, val.g, val.b].map(n => Math.round(n*255).toString(16).padStart(2,'0')).join('');
  const parts = v.name.split('/');
  if (parts.length === 2) { if (!colors[parts[0]]) colors[parts[0]] = {}; colors[parts[0]][parts[1]] = hex; }
  else { colors[v.name.replace(/\//g, '-')] = hex; }
});
return JSON.stringify({ theme: { extend: { colors } } }, null, 2);
