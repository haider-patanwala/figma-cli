// var visualize — params: __args.collection (string|null). Ported verbatim from variables.js.
await figma.loadFontAsync({ family: 'Inter', style: 'Medium' });
await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });

const collections = await figma.variables.getLocalVariableCollectionsAsync();
const colorVars = await figma.variables.getLocalVariablesAsync('COLOR');

const targetCols = (__args.collection ? collections.filter(c => c.name.toLowerCase().includes(String(__args.collection).toLowerCase())) : collections);
if (targetCols.length === 0) return 'No collections found';

// Skip semantic collections (they're aliases, colors already shown in primitives)
const filteredCols = targetCols.filter(c => !c.name.toLowerCase().includes('semantic'));
if (filteredCols.length === 0) return 'No color collections found (only semantic)';

let startX = 0;
figma.currentPage.children.forEach(n => {
  startX = Math.max(startX, n.x + (n.width || 0));
});
startX += 100;

let totalSwatches = 0;

// shadcn color order
const colorOrder = ['slate','gray','zinc','neutral','stone','red','orange','amber','yellow','lime','green','emerald','teal','cyan','sky','blue','indigo','violet','purple','fuchsia','pink','rose','white','black'];

for (const col of filteredCols) {
  const colVars = colorVars.filter(v => v.variableCollectionId === col.id);
  if (colVars.length === 0) continue;

  // Group by prefix (handles both "blue/500" and semantic names)
  const groups = {};
  const semanticGroups = {
    'background': 'base', 'foreground': 'base', 'border': 'base', 'input': 'base', 'ring': 'base',
    'primary': 'primary', 'primary-foreground': 'primary',
    'secondary': 'secondary', 'secondary-foreground': 'secondary',
    'muted': 'muted', 'muted-foreground': 'muted',
    'accent': 'accent', 'accent-foreground': 'accent',
    'card': 'card', 'card-foreground': 'card',
    'popover': 'popover', 'popover-foreground': 'popover',
    'destructive': 'destructive', 'destructive-foreground': 'destructive',
    'chart-1': 'chart', 'chart-2': 'chart', 'chart-3': 'chart', 'chart-4': 'chart', 'chart-5': 'chart',
  };
  colVars.forEach(v => {
    const parts = v.name.split('/');
    let prefix;
    if (parts.length > 1) {
      prefix = parts[0];
    } else if (v.name.startsWith('sidebar-')) {
      prefix = 'sidebar';
    } else {
      prefix = semanticGroups[v.name] || 'other';
    }
    if (!groups[prefix]) groups[prefix] = [];
    groups[prefix].push(v);
  });

  // Sort groups
  const semanticOrder = ['base','primary','secondary','muted','accent','card','popover','destructive','chart','sidebar'];
  const sortedGroups = Object.entries(groups).sort((a, b) => {
    const aColorIdx = colorOrder.indexOf(a[0]);
    const bColorIdx = colorOrder.indexOf(b[0]);
    const aSemanticIdx = semanticOrder.indexOf(a[0]);
    const bSemanticIdx = semanticOrder.indexOf(b[0]);
    if (aColorIdx !== -1 && bColorIdx !== -1) return aColorIdx - bColorIdx;
    if (aColorIdx !== -1) return -1;
    if (bColorIdx !== -1) return 1;
    if (aSemanticIdx !== -1 && bSemanticIdx !== -1) return aSemanticIdx - bSemanticIdx;
    return a[0].localeCompare(b[0]);
  });

  // Create container
  const container = figma.createFrame();
  container.name = col.name;
  container.x = startX;
  container.y = 0;
  container.layoutMode = 'VERTICAL';
  container.primaryAxisSizingMode = 'AUTO';
  container.counterAxisSizingMode = 'AUTO';
  container.itemSpacing = 8;
  container.paddingTop = 32;
  container.paddingBottom = 32;
  container.paddingLeft = 32;
  container.paddingRight = 32;
  container.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }];
  container.cornerRadius = 16;

  // Title
  const title = figma.createText();
  title.characters = col.name;
  title.fontSize = 20;
  title.fontName = { family: 'Inter', style: 'Medium' };
  title.fills = [{ type: 'SOLID', color: { r: 0.1, g: 0.1, b: 0.1 } }];
  container.appendChild(title);

  // Spacer
  const spacer = figma.createFrame();
  spacer.resize(1, 16);
  spacer.fills = [];
  container.appendChild(spacer);

  const modeId = col.modes[0].modeId;
  const swatchesToBind = [];

  for (const [groupName, vars] of sortedGroups) {
    // Row container with label
    const rowContainer = figma.createFrame();
    rowContainer.name = groupName;
    rowContainer.layoutMode = 'HORIZONTAL';
    rowContainer.primaryAxisSizingMode = 'AUTO';
    rowContainer.counterAxisSizingMode = 'AUTO';
    rowContainer.itemSpacing = 16;
    rowContainer.counterAxisAlignItems = 'CENTER';
    rowContainer.fills = [];
    container.appendChild(rowContainer);

    // Label
    const label = figma.createText();
    label.characters = groupName;
    label.fontSize = 13;
    label.fontName = { family: 'Inter', style: 'Medium' };
    label.fills = [{ type: 'SOLID', color: { r: 0.4, g: 0.4, b: 0.4 } }];
    label.resize(80, label.height);
    label.textAlignHorizontal = 'RIGHT';
    rowContainer.appendChild(label);

    // Swatches row
    const swatchRow = figma.createFrame();
    swatchRow.layoutMode = 'HORIZONTAL';
    swatchRow.primaryAxisSizingMode = 'AUTO';
    swatchRow.counterAxisSizingMode = 'AUTO';
    swatchRow.itemSpacing = 0;
    swatchRow.fills = [];
    swatchRow.cornerRadius = 6;
    swatchRow.clipsContent = true;
    rowContainer.appendChild(swatchRow);

    // Sort shades
    vars.sort((a, b) => {
      const aNum = parseInt(a.name.split('/').pop()) || 0;
      const bNum = parseInt(b.name.split('/').pop()) || 0;
      return aNum - bNum;
    });

    for (const v of vars) {
      const swatch = figma.createFrame();
      swatch.name = v.name;
      swatch.resize(48, 32);
      swatch.fills = [{ type: 'SOLID', color: { r: 0.9, g: 0.9, b: 0.9 } }];
      swatchRow.appendChild(swatch);
      swatchesToBind.push({ swatch, variable: v, modeId });
      totalSwatches++;
    }
  }

  // Bind after appending
  for (const { swatch, variable, modeId } of swatchesToBind) {
    try {
      let value = variable.valuesByMode[modeId];
      if (value && value.type === 'VARIABLE_ALIAS') {
        const resolved = figma.variables.getVariableById(value.id);
        if (resolved) value = resolved.valuesByMode[Object.keys(resolved.valuesByMode)[0]];
      }
      if (value && value.r !== undefined) {
        swatch.fills = [figma.variables.setBoundVariableForPaint(
          { type: 'SOLID', color: { r: value.r, g: value.g, b: value.b } }, 'color', variable
        )];
      }
    } catch (e) {}
  }

  startX += container.width + 60;
}

figma.viewport.scrollAndZoomIntoView(figma.currentPage.children.slice(-filteredCols.length));
return 'Created ' + totalSwatches + ' color swatches';
