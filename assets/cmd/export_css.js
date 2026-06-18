// Export local variables as CSS custom properties. No params.
const vars = await figma.variables.getLocalVariablesAsync();
const css = vars.map(v => {
  const val = Object.values(v.valuesByMode)[0];
  if (v.resolvedType === 'COLOR') {
    const hex = '#' + [val.r, val.g, val.b].map(n => Math.round(n*255).toString(16).padStart(2,'0')).join('');
    return '  --' + v.name.replace(/\//g, '-') + ': ' + hex + ';';
  }
  return '  --' + v.name.replace(/\//g, '-') + ': ' + val + (v.resolvedType === 'FLOAT' ? 'px' : '') + ';';
}).join('\n');
return ':root {\n' + css + '\n}';
