//! Small helpers that emit Figma Plugin API JS snippets, ported faithfully from
//! the pure helpers in `src/lib/cli-core.js` (hexToRgb, generateFillCode,
//! generateStrokeCode, varLoadingCode, smartPosCode). Standalone `create`/`set`
//! commands compose these into eval payloads, same as the original CLI.

/// JSON-encode a string for safe interpolation into generated JS.
pub fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

pub fn is_var_ref(v: &str) -> bool {
    v.starts_with("var:")
}

pub fn var_name(v: &str) -> &str {
    v.strip_prefix("var:").unwrap_or(v)
}

/// (r,g,b) floats 0..1 from a hex string (#rgb or #rrggbb).
pub fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let h = hex.trim_start_matches('#');
    let h = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        h.to_string()
    };
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64 / 255.0;
    Some((r, g, b))
}

/// Returns (js, uses_vars). `property` is e.g. "fills".
pub fn fill_code(color: &str, node_var: &str, property: &str) -> (String, bool) {
    if is_var_ref(color) {
        let name = js_string(var_name(color));
        (format!("{node_var}.{property} = [boundFill(vars[{name}])];"), true)
    } else {
        match hex_to_rgb(color) {
            Some((r, g, b)) => (
                format!("{node_var}.{property} = [{{ type: 'SOLID', color: {{ r: {r}, g: {g}, b: {b} }} }}];"),
                false,
            ),
            None => (format!("/* invalid color {} */", js_string(color)), false),
        }
    }
}

/// Returns (js, uses_vars) for a stroke binding.
pub fn stroke_code(color: &str, node_var: &str, weight: f64) -> (String, bool) {
    if is_var_ref(color) {
        let name = js_string(var_name(color));
        (format!("{node_var}.strokes = [boundFill(vars[{name}])]; {node_var}.strokeWeight = {weight};"), true)
    } else {
        match hex_to_rgb(color) {
            Some((r, g, b)) => (
                format!("{node_var}.strokes = [{{ type: 'SOLID', color: {{ r: {r}, g: {g}, b: {b} }} }}]; {node_var}.strokeWeight = {weight};"),
                false,
            ),
            None => (format!("/* invalid color {} */", js_string(color)), false),
        }
    }
}

/// JS that builds a `vars` map from shadcn collections + a `boundFill` helper.
pub fn var_loading_code() -> &'static str {
    r#"
const collections = await figma.variables.getLocalVariableCollectionsAsync();
const vars = {};
for (const col of collections) {
  if (col.name.startsWith('shadcn')) {
    for (const id of col.variableIds) {
      const v = await figma.variables.getVariableByIdAsync(id);
      if (v) vars[v.name] = v;
    }
  }
}
const boundFill = (variable) => figma.variables.setBoundVariableForPaint(
  { type: 'SOLID', color: { r: 0.5, g: 0.5, b: 0.5 } }, 'color', variable
);
"#
}

/// Build the `const nodes = …` selector JS, ported from cli-core
/// `buildNodeSelector`: prefer --node id(s), then --query name match, else the
/// current selection.
pub fn node_selector(node: Option<&str>, query: Option<&str>) -> String {
    if let Some(q) = query {
        let pat = js_string(&q.to_lowercase());
        return format!(
            "const __pat = {pat}; const nodes = figma.currentPage.findAll(n => typeof n.name === 'string' && n.name.toLowerCase().includes(__pat));"
        );
    }
    if let Some(n) = node {
        let ids: Vec<&str> = n.split([' ', ',']).filter(|s| !s.is_empty()).collect();
        if ids.len() == 1 {
            let id = js_string(ids[0]);
            return format!("const __n = await figma.getNodeByIdAsync({id}); const nodes = __n ? [__n] : [];");
        }
        let arr = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into());
        return format!(
            "const __ids = {arr}; const __res = await Promise.all(__ids.map(id => figma.getNodeByIdAsync(id))); const nodes = __res.filter(Boolean);"
        );
    }
    "const nodes = figma.currentPage.selection;".to_string()
}

/// JS that computes `smartX` = next free x position on the page.
pub fn smart_pos_code(gap: f64) -> String {
    format!(
        r#"
const children = figma.currentPage.children;
let smartX = 0;
if (children.length > 0) {{
  children.forEach(n => {{ smartX = Math.max(smartX, n.x + n.width); }});
  smartX += {gap};
}}
"#
    )
}
