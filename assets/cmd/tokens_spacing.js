// Create a 4px-base spacing scale as FLOAT variables. Params: __args.collection.
const spacings = { '0': 0, '0.5': 2, '1': 4, '1.5': 6, '2': 8, '2.5': 10, '3': 12, '3.5': 14, '4': 16, '5': 20, '6': 24, '7': 28, '8': 32, '9': 36, '10': 40, '11': 44, '12': 48, '14': 56, '16': 64, '20': 80, '24': 96, '28': 112, '32': 128, '36': 144, '40': 160, '44': 176, '48': 192 };
const cols = await figma.variables.getLocalVariableCollectionsAsync();
let col = cols.find(c => c.name === __args.collection);
if (!col) col = figma.variables.createVariableCollection(__args.collection);
const modeId = col.modes[0].modeId;
const existingVars = await figma.variables.getLocalVariablesAsync();
let count = 0;
for (const [name, value] of Object.entries(spacings)) {
  if (!existingVars.find(v => v.name === 'spacing/' + name)) {
    const v = figma.variables.createVariable('spacing/' + name, col, 'FLOAT');
    v.setValueForMode(modeId, value);
    count++;
  }
}
return 'Created ' + count + ' spacing variables';
