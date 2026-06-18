// Create a border-radius scale as FLOAT variables. Params: __args.collection.
const radii = { 'none': 0, 'sm': 2, 'default': 4, 'md': 6, 'lg': 8, 'xl': 12, '2xl': 16, '3xl': 24, 'full': 9999 };
const cols = await figma.variables.getLocalVariableCollectionsAsync();
let col = cols.find(c => c.name === __args.collection);
if (!col) col = figma.variables.createVariableCollection(__args.collection);
const modeId = col.modes[0].modeId;
const existingVars = await figma.variables.getLocalVariablesAsync();
let count = 0;
for (const [name, value] of Object.entries(radii)) {
  if (!existingVars.find(v => v.name === 'radius/' + name)) {
    const v = figma.variables.createVariable('radius/' + name, col, 'FLOAT');
    v.setValueForMode(modeId, value);
    count++;
  }
}
return 'Created ' + count + ' radius variables';
