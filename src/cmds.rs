//! JS-payload builders for the ported eval-based commands. Each function
//! returns a Figma Plugin API JS string (the daemon evals it via the plugin),
//! faithfully mirroring the corresponding generator in the original
//! `src/commands/*.js`. Pure string construction — no I/O.

use crate::jsgen::{
    fill_code, is_var_ref, js_string, node_selector, smart_pos_code, stroke_code, var_loading_code,
};

fn wrap_async(body: &str) -> String {
    format!("(async () => {{\n{body}\n}})()")
}

// --------------------------------------------------------------------------
// create <shape>
// --------------------------------------------------------------------------

pub struct ShapeOpts {
    pub name: Option<String>,
    pub width: f64,
    pub height: Option<f64>,
    pub x: Option<f64>,
    pub y: f64,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub radius: Option<f64>,
    pub opacity: Option<f64>,
}

fn smart_x(x: Option<f64>, gap: f64) -> String {
    match x {
        Some(v) => format!("const smartX = {v};"),
        None => smart_pos_code(gap),
    }
}

pub fn create_frame(o: &ShapeOpts) -> String {
    let name = o.name.clone().unwrap_or_else(|| "Frame".into());
    let (fill, uses) = match &o.fill {
        Some(c) => fill_code(c, "frame", "fills"),
        None => (String::new(), false),
    };
    let body = format!(
        "{vars}{pos}\nconst frame = figma.createFrame();\nframe.name = {name};\nframe.x = smartX;\nframe.y = {y};\nframe.resize({w}, {h});\n{fill}\n{radius}\nfigma.currentPage.selection = [frame];\nreturn {{ id: frame.id, name: frame.name, x: smartX, y: {y} }};",
        vars = if uses { var_loading_code() } else { "" },
        pos = smart_x(o.x, 100.0),
        name = js_string(&name),
        y = o.y,
        w = o.width,
        h = o.height.unwrap_or(o.width),
        fill = fill,
        radius = o.radius.map(|r| format!("frame.cornerRadius = {r};")).unwrap_or_default(),
    );
    wrap_async(&body)
}

pub fn create_rect(o: &ShapeOpts) -> String {
    let name = o.name.clone().unwrap_or_else(|| "Rectangle".into());
    let fill_ref = o.fill.clone().unwrap_or_else(|| "#D9D9D9".into());
    let (fill, uses_f) = fill_code(&fill_ref, "rect", "fills");
    let (stroke, uses_s) = match &o.stroke {
        Some(c) => stroke_code(c, "rect", 1.0),
        None => (String::new(), false),
    };
    let body = format!(
        "{vars}{pos}\nconst rect = figma.createRectangle();\nrect.name = {name};\nrect.x = smartX;\nrect.y = {y};\nrect.resize({w}, {h});\n{fill}\n{radius}\n{opacity}\n{stroke}\nfigma.currentPage.selection = [rect];\nreturn {{ id: rect.id, name: rect.name }};",
        vars = if uses_f || uses_s { var_loading_code() } else { "" },
        pos = smart_x(o.x, 100.0),
        name = js_string(&name),
        y = o.y, w = o.width, h = o.height.unwrap_or(o.width),
        fill = fill,
        radius = o.radius.map(|r| format!("rect.cornerRadius = {r};")).unwrap_or_default(),
        opacity = o.opacity.map(|v| format!("rect.opacity = {v};")).unwrap_or_default(),
        stroke = stroke,
    );
    wrap_async(&body)
}

pub fn create_ellipse(o: &ShapeOpts) -> String {
    let name = o.name.clone().unwrap_or_else(|| "Ellipse".into());
    let fill_ref = o.fill.clone().unwrap_or_else(|| "#D9D9D9".into());
    let (fill, uses_f) = fill_code(&fill_ref, "ellipse", "fills");
    let (stroke, uses_s) = match &o.stroke {
        Some(c) => stroke_code(c, "ellipse", 1.0),
        None => (String::new(), false),
    };
    let body = format!(
        "{vars}{pos}\nconst ellipse = figma.createEllipse();\nellipse.name = {name};\nellipse.x = smartX;\nellipse.y = {y};\nellipse.resize({w}, {h});\n{fill}\n{stroke}\nfigma.currentPage.selection = [ellipse];\nreturn {{ id: ellipse.id, name: ellipse.name }};",
        vars = if uses_f || uses_s { var_loading_code() } else { "" },
        pos = smart_x(o.x, 100.0),
        name = js_string(&name),
        y = o.y, w = o.width, h = o.height.unwrap_or(o.width),
        fill = fill, stroke = stroke,
    );
    wrap_async(&body)
}

pub struct TextOpts {
    pub content: String,
    pub x: Option<f64>,
    pub y: f64,
    pub size: f64,
    pub color: String,
    pub weight: String,
    pub font: String,
    pub width: Option<f64>,
    pub spacing: f64,
}

pub fn create_text(o: &TextOpts) -> String {
    let style = match o.weight.to_lowercase().as_str() {
        "medium" => "Medium",
        "semibold" => "Semi Bold",
        "bold" => "Bold",
        _ => "Regular",
    };
    let (fill, uses) = fill_code(&o.color, "text", "fills");
    let body = format!(
        "{vars}{pos}\nawait figma.loadFontAsync({{ family: {font}, style: {style} }});\nconst text = figma.createText();\ntext.fontName = {{ family: {font}, style: {style} }};\ntext.characters = {chars};\ntext.fontSize = {size};\n{fill}\ntext.x = smartX;\ntext.y = {y};\n{wid}\nfigma.currentPage.selection = [text];\nreturn {{ id: text.id, name: text.name }};",
        vars = if uses { var_loading_code() } else { "" },
        pos = smart_x(o.x, o.spacing),
        font = js_string(&o.font), style = js_string(style),
        chars = js_string(&o.content), size = o.size, fill = fill, y = o.y,
        wid = o.width.map(|w| format!("text.resize({w}, text.height); text.textAutoResize = 'HEIGHT';")).unwrap_or_default(),
    );
    wrap_async(&body)
}

pub fn create_component(name: Option<&str>) -> String {
    let n = js_string(name.unwrap_or("Component"));
    let body = format!(
        "const sel = figma.currentPage.selection;\nif (sel.length === 0) return 'No selection';\nif (sel.length === 1) {{ const c = figma.createComponentFromNode(sel[0]); c.name = {n}; figma.currentPage.selection = [c]; return {{ id: c.id, name: c.name }}; }}\nconst g = figma.group(sel, figma.currentPage); const c = figma.createComponentFromNode(g); c.name = {n}; figma.currentPage.selection = [c]; return {{ id: c.id, name: c.name }};"
    );
    wrap_async(&body)
}

pub fn create_group(name: Option<&str>) -> String {
    let n = js_string(name.unwrap_or("Group"));
    let body = format!(
        "const sel = figma.currentPage.selection;\nif (sel.length < 2) return 'Select 2+ elements to group';\nconst g = figma.group(sel, figma.currentPage); g.name = {n}; figma.currentPage.selection = [g]; return {{ id: g.id, name: g.name }};"
    );
    wrap_async(&body)
}

pub struct AutoLayoutOpts {
    pub name: Option<String>,
    pub direction: String,
    pub gap: f64,
    pub padding: f64,
    pub x: Option<f64>,
    pub y: f64,
    pub fill: Option<String>,
    pub radius: Option<f64>,
    pub spacing: f64,
}

pub fn create_autolayout(o: &AutoLayoutOpts) -> String {
    let name = o.name.clone().unwrap_or_else(|| "Auto Layout".into());
    let mode = if o.direction == "col" { "VERTICAL" } else { "HORIZONTAL" };
    let (fill, uses) = match &o.fill {
        Some(c) => fill_code(c, "frame", "fills"),
        None => ("frame.fills = [];".to_string(), false),
    };
    let body = format!(
        "{vars}{pos}\nconst frame = figma.createFrame();\nframe.name = {name};\nframe.x = smartX;\nframe.y = {y};\nframe.layoutMode = {mode};\nframe.primaryAxisSizingMode = 'AUTO';\nframe.counterAxisSizingMode = 'AUTO';\nframe.itemSpacing = {gap};\nframe.paddingTop = {p}; frame.paddingRight = {p}; frame.paddingBottom = {p}; frame.paddingLeft = {p};\n{fill}\n{radius}\nfigma.currentPage.selection = [frame];\nreturn {{ id: frame.id, name: frame.name }};",
        vars = if uses { var_loading_code() } else { "" },
        pos = smart_x(o.x, o.spacing),
        name = js_string(&name), y = o.y, mode = js_string(mode),
        gap = o.gap, p = o.padding, fill = fill,
        radius = o.radius.map(|r| format!("frame.cornerRadius = {r};")).unwrap_or_default(),
    );
    wrap_async(&body)
}

// --------------------------------------------------------------------------
// set <prop> (operate on a node selector)
// --------------------------------------------------------------------------

pub struct Sel {
    pub node: Option<String>,
    pub query: Option<String>,
}
impl Sel {
    fn js(&self) -> String {
        node_selector(self.node.as_deref(), self.query.as_deref())
    }
}

fn set_each(sel: &Sel, mutate: &str, label: &str) -> String {
    let body = format!(
        "{selector}\nif (nodes.length === 0) return 'No node found';\nnodes.forEach(n => {{ {mutate} }});\nreturn '{label} on ' + nodes.length + ' element(s)';",
        selector = sel.js()
    );
    wrap_async(&body)
}

pub fn set_fill(sel: &Sel, color: &str) -> String {
    if is_var_ref(color) {
        let name = js_string(crate::jsgen::var_name(color));
        let body = format!(
            "{selector}\nconst collections = await figma.variables.getLocalVariableCollectionsAsync();\nconst allVars = await figma.variables.getLocalVariablesAsync();\nlet variable = null;\nfor (const v of allVars) {{ if (v.name !== {name}) continue; const col = collections.find(c => c.id === v.variableCollectionId); if (col && col.name.startsWith('shadcn')) {{ variable = v; break; }} }}\nif (!variable) variable = allVars.find(v => v.name === {name});\nif (!variable) return 'Variable not found: ' + {name};\nlet count = 0;\nfor (const n of nodes) {{\n  let targets = ('fills' in n) ? [n] : (typeof n.findAll === 'function' ? n.findAll(c => 'fills' in c) : []);\n  for (const t of targets) {{ const base = (t.fills && t.fills[0]) || {{ type: 'SOLID', color: {{ r:0,g:0,b:0 }} }}; t.fills = [figma.variables.setBoundVariableForPaint(base, 'color', variable)]; count++; }}\n}}\nreturn 'Bound ' + variable.name + ' to fill on ' + count + ' node(s)';",
            selector = sel.js()
        );
        return wrap_async(&body);
    }
    let (r, g, b) = crate::jsgen::hex_to_rgb(color).unwrap_or((0.5, 0.5, 0.5));
    let mutate = format!("let ts = ('fills' in n) ? [n] : (typeof n.findAll === 'function' ? n.findAll(c => 'fills' in c) : []); ts.forEach(t => t.fills = [{{ type: 'SOLID', color: {{ r: {r}, g: {g}, b: {b} }} }}]);");
    set_each(sel, &mutate, "Fill set")
}

pub fn set_stroke(sel: &Sel, color: &str) -> String {
    let (code, _) = stroke_code(color, "n", 1.0);
    set_each(sel, &format!("if ('strokes' in n) {{ {code} }}"), "Stroke set")
}

pub fn set_radius(sel: &Sel, value: f64) -> String {
    set_each(sel, &format!("if ('cornerRadius' in n) n.cornerRadius = {value};"), "Radius set")
}
pub fn set_size(sel: &Sel, w: f64, h: f64) -> String {
    set_each(sel, &format!("if ('resize' in n) n.resize({w}, {h});"), "Size set")
}
pub fn set_pos(sel: &Sel, x: f64, y: f64) -> String {
    set_each(sel, &format!("n.x = {x}; n.y = {y};"), "Position set")
}
pub fn set_opacity(sel: &Sel, v: f64) -> String {
    set_each(sel, &format!("if ('opacity' in n) n.opacity = {v};"), "Opacity set")
}
pub fn set_name(sel: &Sel, name: &str) -> String {
    set_each(sel, &format!("n.name = {};", js_string(name)), "Renamed")
}
pub fn set_text(sel: &Sel, text: &str) -> String {
    let body = format!(
        "{selector}\nif (nodes.length === 0) return 'No node found';\nlet count = 0;\nfor (const n of nodes) {{ if (n.type === 'TEXT') {{ await figma.loadFontAsync(n.fontName); n.characters = {t}; count++; }} }}\nreturn 'Set text on ' + count + ' node(s)';",
        selector = sel.js(), t = js_string(text)
    );
    wrap_async(&body)
}

pub fn set_scale(sel: &Sel, factor: f64, keep_spacing: bool) -> String {
    let scale_spacing = !keep_spacing;
    let body = format!(
        "{selector}\nif (nodes.length === 0) return 'No node found';\nconst scaleSpacing = {scale_spacing};\nconst origin = scaleSpacing && nodes.length > 1 ? {{ x: Math.min(...nodes.map(n => n.x || 0)), y: Math.min(...nodes.map(n => n.y || 0)) }} : null;\nconst orig = nodes.map(n => ({{ x: n.x || 0, y: n.y || 0 }}));\nlet count = 0;\nfor (const n of nodes) {{ if (typeof n.rescale === 'function') {{ n.rescale({factor}); count++; }} else if ('resize' in n) {{ n.resize(n.width * {factor}, n.height * {factor}); count++; }} }}\nif (origin) {{ for (let i = 0; i < nodes.length; i++) {{ const n = nodes[i]; if (typeof n.x !== 'number') continue; n.x = origin.x + (orig[i].x - origin.x) * {factor}; n.y = origin.y + (orig[i].y - origin.y) * {factor}; }} }}\nreturn 'Scaled ' + count + ' element(s) by {factor}';",
        selector = sel.js()
    );
    wrap_async(&body)
}

// --------------------------------------------------------------------------
// sizing / layout shortcuts
// --------------------------------------------------------------------------

pub fn sizing_hug(axis: &str) -> String {
    let h = if axis == "h" || axis == "both" { "if ('layoutSizingHorizontal' in n) n.layoutSizingHorizontal = 'HUG';" } else { "" };
    let v = if axis == "v" || axis == "both" { "if ('layoutSizingVertical' in n) n.layoutSizingVertical = 'HUG';" } else { "" };
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ {h} {v} if (n.layoutMode) {{ n.primaryAxisSizingMode = 'AUTO'; n.counterAxisSizingMode = 'AUTO'; }} }});\nreturn 'Set hug on ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}
pub fn sizing_fill(axis: &str) -> String {
    let h = if axis == "h" || axis == "both" { "if ('layoutSizingHorizontal' in n) n.layoutSizingHorizontal = 'FILL';" } else { "" };
    let v = if axis == "v" || axis == "both" { "if ('layoutSizingVertical' in n) n.layoutSizingVertical = 'FILL';" } else { "" };
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ {h} {v} }});\nreturn 'Set fill on ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}
pub fn sizing_fixed(w: f64, h: f64) -> String {
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ if ('layoutSizingHorizontal' in n) n.layoutSizingHorizontal = 'FIXED'; if ('layoutSizingVertical' in n) n.layoutSizingVertical = 'FIXED'; if ('resize' in n) n.resize({w}, {h}); }});\nreturn 'Set fixed {w}x{h} on ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}
pub fn set_padding(t: f64, r: f64, b: f64, l: f64) -> String {
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ if ('paddingTop' in n) {{ n.paddingTop = {t}; n.paddingRight = {r}; n.paddingBottom = {b}; n.paddingLeft = {l}; }} }});\nreturn 'Set padding on ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}
pub fn set_gap(value: f64) -> String {
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ if ('itemSpacing' in n) n.itemSpacing = {value}; }});\nreturn 'Set gap {value} on ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}
pub fn align(alignment: &str) -> String {
    let val = match alignment.to_lowercase().as_str() {
        "start" => "MIN", "end" => "MAX", "stretch" => "STRETCH", _ => "CENTER",
    };
    let body = format!("const nodes = figma.currentPage.selection;\nif (nodes.length === 0) return 'No selection';\nnodes.forEach(n => {{ if ('primaryAxisAlignItems' in n) n.primaryAxisAlignItems = '{val}'; if ('counterAxisAlignItems' in n) n.counterAxisAlignItems = '{val}'; }});\nreturn 'Aligned ' + nodes.length + ' element(s)';");
    wrap_async(&body)
}

// --------------------------------------------------------------------------
// bind <prop> <varName>
// --------------------------------------------------------------------------

fn bind_lookup(sel: &Sel, var_name: &str) -> String {
    format!(
        "{selector}\nconst vars = await figma.variables.getLocalVariablesAsync();\nconst v = vars.find(v => v.name === {name} || v.name.endsWith({slash}));\nif (!v) return 'Variable not found: ' + {name};\nif (nodes.length === 0) return 'No node selected';",
        selector = sel.js(), name = js_string(var_name), slash = js_string(&format!("/{var_name}"))
    )
}
pub fn bind_fill(sel: &Sel, var_name: &str) -> String {
    let body = format!("{lk}\nnodes.forEach(n => {{ if ('fills' in n && n.fills.length > 0) n.fills = [figma.variables.setBoundVariableForPaint(n.fills[0], 'color', v)]; }});\nreturn 'Bound ' + v.name + ' to fill on ' + nodes.length + ' element(s)';", lk = bind_lookup(sel, var_name));
    wrap_async(&body)
}
pub fn bind_stroke(sel: &Sel, var_name: &str) -> String {
    let body = format!("{lk}\nnodes.forEach(n => {{ if ('strokes' in n) {{ const s = n.strokes[0] || {{ type: 'SOLID', color: {{r:0,g:0,b:0}} }}; n.strokes = [figma.variables.setBoundVariableForPaint(s, 'color', v)]; }} }});\nreturn 'Bound ' + v.name + ' to stroke on ' + nodes.length + ' element(s)';", lk = bind_lookup(sel, var_name));
    wrap_async(&body)
}
pub fn bind_prop(sel: &Sel, var_name: &str, field: &str) -> String {
    let body = format!("{lk}\nnodes.forEach(n => {{ if ('{field}' in n) n.setBoundVariable('{field}', v); }});\nreturn 'Bound ' + v.name + ' to {field} on ' + nodes.length + ' element(s)';", lk = bind_lookup(sel, var_name));
    wrap_async(&body)
}
pub fn bind_padding(sel: &Sel, var_name: &str, side: &str) -> String {
    let sides = if side == "all" {
        "['paddingTop','paddingRight','paddingBottom','paddingLeft']".to_string()
    } else {
        let cap = format!("padding{}{}", side[..1].to_uppercase(), &side[1..]);
        format!("[{}]", js_string(&cap))
    };
    let body = format!("{lk}\nconst sides = {sides};\nnodes.forEach(n => {{ sides.forEach(s => {{ if (s in n) n.setBoundVariable(s, v); }}); }});\nreturn 'Bound ' + v.name + ' to padding on ' + nodes.length + ' element(s)';", lk = bind_lookup(sel, var_name));
    wrap_async(&body)
}

// --------------------------------------------------------------------------
// select / delete / duplicate / get
// --------------------------------------------------------------------------

pub fn select(node_id: &str) -> String {
    let id = js_string(node_id);
    let body = format!("const n = await figma.getNodeByIdAsync({id}); if (!n) return 'Node not found'; figma.currentPage.selection = [n]; figma.viewport.scrollAndZoomIntoView([n]); return {{ id: n.id, name: n.name }};");
    wrap_async(&body)
}
pub fn delete(node_id: Option<&str>) -> String {
    match node_id {
        Some(id) => {
            let id = js_string(id);
            wrap_async(&format!("const n = await figma.getNodeByIdAsync({id}); if (!n) return 'Node not found'; n.remove(); return 'Deleted ' + {id};"))
        }
        None => wrap_async("const sel = figma.currentPage.selection; if (sel.length === 0) return 'No selection'; const c = sel.length; sel.forEach(n => n.remove()); return 'Deleted ' + c + ' element(s)';"),
    }
}
pub fn duplicate(node_id: Option<&str>, offset: f64) -> String {
    match node_id {
        Some(id) => {
            let id = js_string(id);
            wrap_async(&format!("const n = await figma.getNodeByIdAsync({id}); if (!n) return 'Node not found'; const c = n.clone(); c.x += {offset}; c.y += {offset}; figma.currentPage.selection = [c]; return {{ id: c.id }};"))
        }
        None => wrap_async(&format!("const sel = figma.currentPage.selection; if (sel.length === 0) return 'No selection'; const clones = sel.map(n => {{ const c = n.clone(); c.x += {offset}; c.y += {offset}; return c; }}); figma.currentPage.selection = clones; return 'Duplicated ' + clones.length + ' element(s)';")),
    }
}
pub fn get(node_id: Option<&str>) -> String {
    let lookup = match node_id {
        Some(id) => format!("const n = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const n = figma.currentPage.selection[0];".to_string(),
    };
    wrap_async(&format!("{lookup}\nif (!n) return 'No node';\nreturn {{ id: n.id, name: n.name, type: n.type, x: n.x, y: n.y, width: n.width, height: n.height, visible: n.visible, layoutMode: n.layoutMode }};"))
}
