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
// --------------------------------------------------------------------------
// gradient apply
// --------------------------------------------------------------------------

/// Set a node's fills to a single (gradient) paint.
pub fn apply_paint(node_id: &str, paint: &serde_json::Value) -> String {
    wrap_async(&format!(
        "await figma.loadAllPagesAsync();\nconst id = {id};\nlet n = /^selected$/i.test(id) ? figma.currentPage.selection[0] : await figma.getNodeByIdAsync(id);\nif (!n) throw new Error('Node not found: ' + id);\nif (!('fills' in n)) throw new Error('Node does not support fills: ' + n.type);\nn.fills = [{paint}];\nreturn {{ name: n.name, type: n.type }};",
        id = js_string(node_id), paint = paint
    ))
}

/// Build (or populate) a mesh-gradient frame from a recipe { base, blobs, blurFraction }.
/// When `apply_to` is None, creates a new W×H frame; else populates the target frame.
pub fn apply_mesh_wallpaper(apply_to: Option<&str>, recipe: &serde_json::Value, w: f64, h: f64, name: &str) -> String {
    let base = recipe.get("base").cloned().unwrap_or(serde_json::Value::String("#000000".into()));
    let blobs = recipe.get("blobs").cloned().unwrap_or(serde_json::Value::Array(vec![]));
    let blur_frac = recipe.get("blurFraction").and_then(|v| v.as_f64()).unwrap_or(0.42);
    let target = match apply_to {
        Some(id) => format!(
            "const id = {id};\nif (/^selected$/i.test(id)) {{ __target = figma.currentPage.selection[0]; if (!__target) throw new Error('Nothing selected'); }} else {{ __target = await figma.getNodeByIdAsync(id); if (!__target) throw new Error('Node not found: ' + id); }}\nif (__target.type !== 'FRAME') throw new Error('Mesh requires a FRAME target; got ' + __target.type);\nfor (const c of [...__target.children]) c.remove();",
            id = js_string(id)
        ),
        None => format!(
            "__target = figma.createFrame();\n__target.name = {name};\n__target.resize({w}, {h});\nlet __x = 0; figma.currentPage.children.forEach(n => {{ __x = Math.max(__x, n.x + (n.width || 0)); }});\n__target.x = __x + 100; __target.y = 0;",
            name = js_string(name)
        ),
    };
    wrap_async(&format!(
        "await figma.loadAllPagesAsync();\nconst __hex = (h) => {{ h = h.replace('#',''); return {{ r: parseInt(h.slice(0,2),16)/255, g: parseInt(h.slice(2,4),16)/255, b: parseInt(h.slice(4,6),16)/255 }}; }};\nlet __target;\n{target}\nconst W = __target.width, H = __target.height, D = Math.min(W, H);\n__target.clipsContent = true;\n__target.fills = [{{ type:'SOLID', color: __hex({base}), opacity:1, visible:true, blendMode:'NORMAL' }}];\nconst __blobs = {blobs};\nconst __blur = Math.round(D * {blur_frac});\nfor (const b of __blobs) {{ const e = figma.createEllipse(); const R = Math.round(D * b.r); e.resize(R*2, R*2); e.x = b.fx*W - R; e.y = b.fy*H - R; e.fills = [{{ type:'SOLID', color: __hex(b.color), opacity:1, visible:true, blendMode:'NORMAL' }}]; e.effects = [{{ type:'LAYER_BLUR', radius: Math.round(__blur * (b.blurMul || 1)), visible: true }}]; __target.appendChild(e); }}\nfigma.viewport.scrollAndZoomIntoView([__target]);\nreturn {{ id: __target.id, name: __target.name, blobs: __blobs.length, blur: __blur, w: W, h: H }};",
        base = base, blobs = blobs
    ))
}

// --------------------------------------------------------------------------
// figjam (runs in the same plugin context; FigJam editor)
// --------------------------------------------------------------------------

pub fn figjam_sticky(text: &str, x: f64, y: f64, color: Option<&str>) -> String {
    let fill = match color.and_then(crate::jsgen::hex_to_rgb) {
        Some((r, g, b)) => format!("sticky.fills = [{{ type: 'SOLID', color: {{ r: {r}, g: {g}, b: {b} }} }}];"),
        None => String::new(),
    };
    wrap_async(&format!(
        "const sticky = figma.createSticky();\nsticky.x = {x}; sticky.y = {y};\n{fill}\nawait figma.loadFontAsync({{ family: 'Inter', style: 'Medium' }});\nsticky.text.characters = {t};\nreturn {{ id: sticky.id, x: sticky.x, y: sticky.y }};",
        t = js_string(text)
    ))
}
pub fn figjam_shape(text: &str, x: f64, y: f64, w: f64, h: f64, shape_type: &str) -> String {
    wrap_async(&format!(
        "const shape = figma.createShapeWithText();\nshape.shapeType = {st};\nshape.x = {x}; shape.y = {y};\nshape.resize({w}, {h});\nif (shape.text) {{ await figma.loadFontAsync({{ family: 'Inter', style: 'Medium' }}); shape.text.characters = {t}; }}\nreturn {{ id: shape.id, x: shape.x, y: shape.y }};",
        st = js_string(shape_type), t = js_string(text)
    ))
}
pub fn figjam_text(content: &str, x: f64, y: f64, size: f64) -> String {
    wrap_async(&format!(
        "const textNode = figma.createText();\ntextNode.x = {x}; textNode.y = {y};\nawait figma.loadFontAsync({{ family: 'Inter', style: 'Medium' }});\ntextNode.fontName = {{ family: 'Inter', style: 'Medium' }};\ntextNode.characters = {t};\ntextNode.fontSize = {size};\nreturn {{ id: textNode.id, x: textNode.x, y: textNode.y }};",
        t = js_string(content)
    ))
}
pub fn figjam_connect(start_id: &str, end_id: &str) -> String {
    wrap_async(&format!(
        "const startNode = await figma.getNodeByIdAsync({s});\nconst endNode = await figma.getNodeByIdAsync({e});\nif (!startNode || !endNode) return {{ error: 'Node not found' }};\nconst connector = figma.createConnector();\nconnector.connectorStart = {{ endpointNodeId: startNode.id, magnet: 'AUTO' }};\nconnector.connectorEnd = {{ endpointNodeId: endNode.id, magnet: 'AUTO' }};\nreturn {{ id: connector.id }};",
        s = js_string(start_id), e = js_string(end_id)
    ))
}
pub fn figjam_move(node_id: &str, x: f64, y: f64) -> String {
    wrap_async(&format!(
        "const n = await figma.getNodeByIdAsync({id}); if (!n) return {{ error: 'Node not found' }}; n.x = {x}; n.y = {y}; return {{ id: n.id, x: n.x, y: n.y }};",
        id = js_string(node_id)
    ))
}
pub fn figjam_update(node_id: &str, text: &str) -> String {
    wrap_async(&format!(
        "const n = await figma.getNodeByIdAsync({id}); if (!n) return {{ error: 'Node not found' }}; await figma.loadFontAsync({{ family: 'Inter', style: 'Medium' }}); if (n.text) n.text.characters = {t}; else if (n.type === 'TEXT') n.characters = {t}; return {{ id: n.id }};",
        id = js_string(node_id), t = js_string(text)
    ))
}
pub fn figjam_info() -> &'static str {
    "(async () => { const p = figma.currentPage; return { name: p.name, id: p.id, childCount: p.children.length }; })()"
}
pub fn figjam_nodes(limit: u32) -> String {
    wrap_async(&format!(
        "return figma.currentPage.children.slice(0, {limit}).map(n => ({{ id: n.id, type: n.type, name: n.name, x: Math.round(n.x), y: Math.round(n.y) }}));"
    ))
}

// --------------------------------------------------------------------------
// variables: create / find
// --------------------------------------------------------------------------

pub fn var_create(name: &str, collection: &str, vtype: &str, value: Option<&str>) -> String {
    let t = vtype.to_uppercase();
    let set_val = match value {
        Some(val) => {
            let v = js_string(val);
            format!("let figmaValue = {v}; if ({tj} === 'COLOR') figmaValue = hexToRgb({v}); else if ({tj} === 'FLOAT') figmaValue = parseFloat({v}); else if ({tj} === 'BOOLEAN') figmaValue = {v} === 'true'; v.setValueForMode(modeId, figmaValue);", tj = js_string(&t))
        }
        None => String::new(),
    };
    wrap_async(&format!(
        "const cols = await figma.variables.getLocalVariableCollectionsAsync();\nlet col = cols.find(c => c.id === {c} || c.name === {c});\nif (!col) return 'Collection not found: ' + {c};\nconst modeId = col.modes[0].modeId;\nfunction hexToRgb(hex) {{ const r = /^#?([a-f\\d]{{2}})([a-f\\d]{{2}})([a-f\\d]{{2}})$/i.exec(hex); return r ? {{ r: parseInt(r[1],16)/255, g: parseInt(r[2],16)/255, b: parseInt(r[3],16)/255 }} : null; }}\nconst v = figma.variables.createVariable({n}, col, {t});\n{set_val}\nreturn {{ id: v.id, name: v.name }};",
        c = js_string(collection), n = js_string(name), t = js_string(&t)
    ))
}

pub fn var_create_batch(collection: &str, vars_json: &str) -> String {
    wrap_async(&format!(
        "const vars = {vars_json};\nconst cols = await figma.variables.getLocalVariableCollectionsAsync();\nlet col = cols.find(c => c.id === {c} || c.name === {c});\nif (!col) return 'Collection not found: ' + {c};\nconst modeId = col.modes[0].modeId;\nfunction hexToRgb(hex) {{ const r = /^#?([a-f\\d]{{2}})([a-f\\d]{{2}})([a-f\\d]{{2}})$/i.exec(hex); return r ? {{ r: parseInt(r[1],16)/255, g: parseInt(r[2],16)/255, b: parseInt(r[3],16)/255 }} : null; }}\nlet created = 0;\nfor (const v of vars) {{ const type = (v.type || 'COLOR').toUpperCase(); const variable = figma.variables.createVariable(v.name, col, type); if (v.value !== undefined) {{ let fv = v.value; if (type === 'COLOR') fv = hexToRgb(v.value); else if (type === 'FLOAT') fv = parseFloat(v.value); else if (type === 'BOOLEAN') fv = v.value === true || v.value === 'true'; variable.setValueForMode(modeId, fv); }} created++; }}\nreturn 'Created ' + created + ' variables';",
        c = js_string(collection)
    ))
}

pub fn delete_batch(ids_json: &str) -> String {
    wrap_async(&format!(
        "const ids = {ids_json};\nlet deleted = 0;\nfor (const id of ids) {{ const n = await figma.getNodeByIdAsync(id); if (n) {{ n.remove(); deleted++; }} }}\nreturn 'Deleted ' + deleted + ' nodes';"
    ))
}

pub fn bind_batch(bindings_json: &str) -> String {
    wrap_async(&format!(
        "const bindings = {bindings_json};\nconst vars = await figma.variables.getLocalVariablesAsync();\nlet bound = 0;\nfor (const b of bindings) {{\n  const node = await figma.getNodeByIdAsync(b.nodeId);\n  if (!node) continue;\n  const variable = vars.find(v => v.name === b.variable || v.name.endsWith('/' + b.variable));\n  if (!variable) continue;\n  const prop = b.property.toLowerCase();\n  if (prop === 'fill' && 'fills' in node && node.fills.length > 0) {{ node.fills = [figma.variables.setBoundVariableForPaint(node.fills[0], 'color', variable)]; bound++; }}\n  else if (prop === 'stroke' && 'strokes' in node && node.strokes.length > 0) {{ node.strokes = [figma.variables.setBoundVariableForPaint(node.strokes[0], 'color', variable)]; bound++; }}\n  else if (prop === 'radius' && 'cornerRadius' in node) {{ node.setBoundVariable('cornerRadius', variable); bound++; }}\n  else if (prop === 'gap' && 'itemSpacing' in node) {{ node.setBoundVariable('itemSpacing', variable); bound++; }}\n  else if (prop === 'padding' && 'paddingTop' in node) {{ node.setBoundVariable('paddingTop', variable); node.setBoundVariable('paddingBottom', variable); node.setBoundVariable('paddingLeft', variable); node.setBoundVariable('paddingRight', variable); bound++; }}\n}}\nreturn 'Bound ' + bound + ' properties';"
    ))
}

pub fn rename_batch(pairs_json: &str) -> String {
    wrap_async(&format!(
        "const pairs = {pairs_json};\nlet renamed = 0;\nconst notFound = [];\nfor (const p of pairs) {{ const node = await figma.getNodeByIdAsync(p.id); if (node) {{ node.name = p.name; renamed++; }} else notFound.push(p.id); }}\nreturn {{ renamed, notFound }};"
    ))
}

pub fn var_find(pattern: &str) -> String {
    wrap_async(&format!(
        "const vars = await figma.variables.getLocalVariablesAsync();\nconst p = {p}.toLowerCase();\nreturn vars.filter(v => v.name.toLowerCase().includes(p)).map(v => ({{ name: v.name, type: v.resolvedType }}));",
        p = js_string(pattern)
    ))
}

// --------------------------------------------------------------------------
// slots
// --------------------------------------------------------------------------

pub fn slot_create(name: &str, flex: &str, gap: f64, padding: f64) -> String {
    let mode = if flex == "row" { "HORIZONTAL" } else { "VERTICAL" };
    let body = format!(
        "const sel = figma.currentPage.selection;\nif (sel.length === 0) return {{ error: 'No component selected' }};\nconst comp = sel[0];\nif (comp.type !== 'COMPONENT' && comp.type !== 'COMPONENT_SET') return {{ error: 'Selected node is not a component' }};\nconst slot = comp.createSlot({name});\nslot.layoutMode = {mode};\nslot.itemSpacing = {gap};\nslot.paddingTop = {p}; slot.paddingBottom = {p}; slot.paddingLeft = {p}; slot.paddingRight = {p};\nreturn {{ success: true, slotId: slot.id, slotName: slot.name, componentName: comp.name }};",
        name = js_string(name), mode = js_string(mode), gap = gap, p = padding
    );
    wrap_async(&body)
}

pub fn slot_list(node_id: Option<&str>) -> String {
    let lookup = match node_id {
        Some(id) => format!("const comp = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const sel = figma.currentPage.selection; if (sel.length === 0) return { error: 'No component selected' }; const comp = sel[0];".to_string(),
    };
    wrap_async(&format!(
        "{lookup}\nif (!comp || (comp.type !== 'COMPONENT' && comp.type !== 'COMPONENT_SET')) return {{ error: 'Node is not a component' }};\nfunction findSlots(n, out) {{ if (n.type === 'SLOT') out.push({{ id: n.id, name: n.name }}); if ('children' in n) n.children.forEach(c => findSlots(c, out)); }}\nconst slots = []; findSlots(comp, slots);\nreturn {{ component: comp.name, slots }};"
    ))
}

pub fn slot_reset(node_id: Option<&str>) -> String {
    let lookup = match node_id {
        Some(id) => format!("let node = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const sel = figma.currentPage.selection; if (sel.length === 0) return { error: 'No slot selected' }; let node = sel[0];".to_string(),
    };
    wrap_async(&format!(
        "{lookup}\nif (node.type !== 'SLOT') {{ if (node.type === 'INSTANCE') {{ const slots = node.children.filter(c => c.type === 'SLOT'); if (slots.length === 0) return {{ error: 'No slots found in instance' }}; if (slots.length === 1) node = slots[0]; else return {{ error: 'Multiple slots found' }}; }} else return {{ error: 'Node is not a slot' }}; }}\nconst beforeCount = node.children.length; node.resetSlot(); return {{ success: true, slotName: node.name, beforeCount, afterCount: node.children.length }};"
    ))
}

pub fn slot_convert(node_id: Option<&str>, name: &str) -> String {
    let lookup = match node_id {
        Some(id) => format!("let frame = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const sel = figma.currentPage.selection; if (sel.length === 0) return { error: 'No frame selected' }; let frame = sel[0];".to_string(),
    };
    wrap_async(&format!(
        "{lookup}\nif (frame.type !== 'FRAME') return {{ error: 'Node is not a frame' }};\nlet parent = frame.parent, component = null;\nwhile (parent) {{ if (parent.type === 'COMPONENT' || parent.type === 'COMPONENT_SET') {{ component = parent; break; }} parent = parent.parent; }}\nif (!component) return {{ error: 'Frame is not inside a component' }};\nconst fp = {{ x: frame.x, y: frame.y, width: frame.width, height: frame.height, layoutMode: frame.layoutMode, itemSpacing: frame.itemSpacing, paddingTop: frame.paddingTop, paddingBottom: frame.paddingBottom, paddingLeft: frame.paddingLeft, paddingRight: frame.paddingRight, fills: frame.fills, children: [...frame.children] }};\nconst slot = component.createSlot({name});\nslot.layoutMode = fp.layoutMode; slot.itemSpacing = fp.itemSpacing; slot.paddingTop = fp.paddingTop; slot.paddingBottom = fp.paddingBottom; slot.paddingLeft = fp.paddingLeft; slot.paddingRight = fp.paddingRight; slot.fills = fp.fills; slot.resize(fp.width, fp.height); slot.x = fp.x; slot.y = fp.y;\nfp.children.forEach(c => slot.appendChild(c));\nframe.remove();\nreturn {{ success: true, slotId: slot.id, slotName: slot.name, componentName: component.name }};",
        name = js_string(name)
    ))
}

// --------------------------------------------------------------------------
// variants / prop combine
// --------------------------------------------------------------------------

pub fn variants_from(ids: &[String], property: &str, values: &[String], set_name: &str) -> String {
    let ids_j = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    let vals_j = serde_json::to_string(values).unwrap_or_else(|_| "[]".into());
    wrap_async(&format!(
        "const ids = {ids_j};\nconst values = {vals_j};\nconst property = {property};\nconst setNameArg = {set_name};\nconst components = []; const promoted = []; let baseName = setNameArg;\nfor (let i = 0; i < ids.length; i++) {{\n  const node = await figma.getNodeByIdAsync(ids[i]);\n  if (!node) return {{ error: 'Node not found: ' + ids[i] }};\n  let comp;\n  if (node.type === 'COMPONENT') {{ if (node.parent && node.parent.type === 'COMPONENT_SET') return {{ error: 'Node ' + ids[i] + ' is already a variant' }}; comp = node; }}\n  else if (node.type === 'FRAME' || node.type === 'GROUP') {{ comp = figma.createComponentFromNode(node); promoted.push(ids[i]); }}\n  else return {{ error: 'Unsupported type for ' + ids[i] + ': ' + node.type }};\n  if (!baseName) {{ let n = (comp.name || 'Component'); n = n.replace(/\\s*,\\s*[^,=]+=[^,]+(?:,\\s*[^,=]+=[^,]+)*\\s*$/, ''); n = n.replace(/\\s*\\/.*$/, ''); baseName = n.trim() || 'Component'; }}\n  components.push({{ comp, value: values[i] }});\n}}\nfor (const {{ comp, value }} of components) comp.name = property + '=' + value;\nconst page = figma.currentPage;\nfor (const {{ comp }} of components) if (comp.parent !== page) page.appendChild(comp);\nlet set;\ntry {{ set = figma.combineAsVariants(components.map(c => c.comp), page); }} catch (err) {{ return {{ error: 'combineAsVariants failed: ' + (err && err.message ? err.message : String(err)) }}; }}\nset.name = baseName;\nfigma.currentPage.selection = [set]; figma.viewport.scrollAndZoomIntoView([set]);\nreturn {{ id: set.id, name: set.name, property, values, promotedCount: promoted.length, count: components.length, variantIds: components.map(c => c.comp.id) }};",
        property = js_string(property), set_name = js_string(set_name)
    ))
}

pub fn prop_combine(ids: &[String], name: &str) -> String {
    let ids_j = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    wrap_async(&format!(
        "const components = [];\nfor (const id of {ids_j}) {{ const n = await figma.getNodeByIdAsync(id); if (!n) throw new Error('Node not found: ' + id); if (n.type !== 'COMPONENT') throw new Error('Not a component: ' + id + ' (type=' + n.type + ')'); components.push(n); }}\nconst set = figma.combineAsVariants(components, figma.currentPage);\nset.name = {name};\nreturn {{ id: set.id, name: set.name, count: components.length }};",
        name = js_string(name)
    ))
}

// --------------------------------------------------------------------------
// analyze / lint (param-free IIFEs, ported verbatim from analyze.js)
// --------------------------------------------------------------------------

pub fn lint() -> &'static str {
    r#"(async () => {
  const issues = [];
  function checkNode(node) {
    if (node.name.startsWith('Frame') || node.name.startsWith('Rectangle') || node.name.startsWith('Group')) issues.push({ type: 'naming', severity: 'warning', node: node.id, name: node.name, message: 'Generic name, consider renaming' });
    if (node.fills && Array.isArray(node.fills)) { const hasFillBinding = node.boundVariables && node.boundVariables.fills; if (!hasFillBinding && node.fills.some(f => f.type === 'SOLID')) issues.push({ type: 'color', severity: 'info', node: node.id, name: node.name, message: 'Hardcoded fill color' }); }
    if (node.type === 'TEXT' && !node.textStyleId) issues.push({ type: 'typography', severity: 'info', node: node.id, name: node.name, message: 'Text without style' });
    if (node.type === 'TEXT' && node.fontSize < 12) issues.push({ type: 'accessibility', severity: 'warning', node: node.id, name: node.name, message: 'Text size < 12px may be hard to read' });
    if ('children' in node) node.children.forEach(c => checkNode(c));
  }
  figma.currentPage.children.forEach(c => checkNode(c));
  return { total: issues.length, issues: issues.slice(0, 50) };
})()"#
}

pub fn analyze_colors() -> &'static str {
    r#"(async () => {
  const colors = new Map();
  function rgbToHex(r, g, b) { return '#' + [r, g, b].map(x => Math.round(x * 255).toString(16).padStart(2, '0')).join(''); }
  function checkNode(node) {
    if (node.fills && Array.isArray(node.fills)) node.fills.forEach(f => { if (f.type === 'SOLID' && f.color) { const hex = rgbToHex(f.color.r, f.color.g, f.color.b); colors.set(hex, (colors.get(hex) || 0) + 1); } });
    if ('children' in node) node.children.forEach(c => checkNode(c));
  }
  figma.currentPage.children.forEach(c => checkNode(c));
  return Array.from(colors.entries()).sort((a, b) => b[1] - a[1]).slice(0, 20).map(([hex, count]) => ({ hex, count }));
})()"#
}

pub fn analyze_typography() -> &'static str {
    r#"(async () => {
  const styles = new Map();
  function checkNode(node) {
    if (node.type === 'TEXT') { const key = node.fontName.family + '/' + node.fontSize + '/' + node.fontName.style; styles.set(key, (styles.get(key) || 0) + 1); }
    if ('children' in node) node.children.forEach(c => checkNode(c));
  }
  figma.currentPage.children.forEach(c => checkNode(c));
  return Array.from(styles.entries()).sort((a, b) => b[1] - a[1]).slice(0, 15).map(([key, count]) => { const [family, size, style] = key.split('/'); return { family, size: parseInt(size), style, count }; });
})()"#
}

pub fn analyze_spacing() -> &'static str {
    r#"(async () => {
  const gaps = new Map(), paddings = new Map();
  function checkNode(node) {
    if (node.layoutMode && node.layoutMode !== 'NONE') {
      if (node.itemSpacing !== undefined) gaps.set(node.itemSpacing, (gaps.get(node.itemSpacing) || 0) + 1);
      [node.paddingTop, node.paddingRight, node.paddingBottom, node.paddingLeft].filter(x => x > 0).forEach(v => paddings.set(v, (paddings.get(v) || 0) + 1));
    }
    if ('children' in node) node.children.forEach(c => checkNode(c));
  }
  figma.currentPage.children.forEach(c => checkNode(c));
  return { gaps: Array.from(gaps.entries()).sort((a, b) => b[1] - a[1]).slice(0, 10).map(([v, c]) => ({ value: v, count: c })), paddings: Array.from(paddings.entries()).sort((a, b) => b[1] - a[1]).slice(0, 10).map(([v, c]) => ({ value: v, count: c })) };
})()"#
}

pub fn analyze_clusters() -> &'static str {
    r#"(async () => {
  const patterns = new Map();
  function getSignature(node) { if (node.type === 'FRAME' || node.type === 'GROUP') { const ct = ('children' in node) ? node.children.map(c => c.type).sort().join(',') : ''; return node.type + ':' + ct; } return node.type; }
  function checkNode(node) { if (node.type === 'FRAME' || node.type === 'GROUP') { const sig = getSignature(node); if (!patterns.has(sig)) patterns.set(sig, []); patterns.get(sig).push({ id: node.id, name: node.name }); } if ('children' in node) node.children.forEach(c => checkNode(c)); }
  figma.currentPage.children.forEach(c => checkNode(c));
  return Array.from(patterns.entries()).filter(([_, nodes]) => nodes.length >= 2).sort((a, b) => b[1].length - a[1].length).slice(0, 10).map(([sig, nodes]) => ({ pattern: sig, count: nodes.length, examples: nodes.slice(0, 3) }));
})()"#
}

pub fn node_tree(node_id: Option<&str>, depth: u32) -> String {
    let target = match node_id {
        Some(id) => format!("await figma.getNodeByIdAsync({})", js_string(id)),
        None => "figma.currentPage".to_string(),
    };
    let body = format!(
        "const maxDepth = {depth};\nconst root = {target};\nif (!root) return 'Node not found';\nconst lines = [];\nfunction printNode(node, indent = 0, d = 0) {{ if (d > maxDepth) return; const prefix = '  '.repeat(indent); const size = node.width && node.height ? ' (' + Math.round(node.width) + 'x' + Math.round(node.height) + ')' : ''; lines.push(prefix + node.type + ': ' + node.name + size); if ('children' in node && d < maxDepth) node.children.forEach(c => printNode(c, indent + 1, d + 1)); }}\nprintNode(root);\nreturn lines.join('\\n');"
    );
    wrap_async(&body)
}

pub fn node_bindings(node_id: Option<&str>) -> String {
    let nodes = match node_id {
        Some(id) => format!("[await figma.getNodeByIdAsync({})]", js_string(id)),
        None => "figma.currentPage.selection".to_string(),
    };
    let body = format!(
        "const nodes = {nodes};\nif (!nodes.length) return 'No node selected';\nconst results = [];\nfor (const node of nodes) {{ if (!node) continue; const bindings = {{}}; if (node.boundVariables) {{ for (const [prop, binding] of Object.entries(node.boundVariables)) {{ const b = Array.isArray(binding) ? binding[0] : binding; if (b && b.id) {{ const v = await figma.variables.getVariableByIdAsync(b.id); bindings[prop] = v ? v.name : b.id; }} }} }} results.push({{ id: node.id, name: node.name, bindings }}); }}\nreturn results;"
    );
    wrap_async(&body)
}

pub fn get(node_id: Option<&str>) -> String {
    let lookup = match node_id {
        Some(id) => format!("const n = await figma.getNodeByIdAsync({});", js_string(id)),
        None => "const n = figma.currentPage.selection[0];".to_string(),
    };
    wrap_async(&format!("{lookup}\nif (!n) return 'No node';\nreturn {{ id: n.id, name: n.name, type: n.type, x: n.x, y: n.y, width: n.width, height: n.height, visible: n.visible, layoutMode: n.layoutMode }};"))
}
