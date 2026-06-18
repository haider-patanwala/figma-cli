(() => {
  // tools-src/pngjs.stub.js
  var PNG = { sync: { read() {
    throw new Error("decode in host");
  } } };

  // tools-src/jpegjs.stub.js
  function decode() {
    throw new Error("decode in host");
  }
  var jpegjs_stub_default = { decode };

  // tools-src/fs.stub.js
  function readFileSync() {
    throw new Error("fs unavailable in tools engine");
  }

  // tools-src/path.stub.js
  function extname(p) {
    const m = String(p).match(/\.[^.\/]+$/);
    return m ? m[0] : "";
  }

  // tools-src/gradient-extractor.js
  function loadImage(path) {
    const buf = readFileSync(path);
    const ext = extname(path).toLowerCase();
    if (ext === ".png") {
      const png = PNG.sync.read(buf);
      return { width: png.width, height: png.height, data: png.data };
    }
    if (ext === ".jpg" || ext === ".jpeg") {
      const j = jpegjs_stub_default.decode(buf, { useTArray: true });
      return { width: j.width, height: j.height, data: j.data };
    }
    throw new Error(`Unsupported image format "${ext}". Use PNG or JPG.`);
  }
  function pixelAt(img, x, y) {
    const i = (y * img.width + x) * 4;
    return [img.data[i], img.data[i + 1], img.data[i + 2], img.data[i + 3]];
  }
  function detectInnerBox(img) {
    const { width: w, height: h } = img;
    const isBorder = (r, g, b, a) => a === 0 || r + g + b < 30;
    const rowAllBorder = (y) => {
      for (let x = 0; x < w; x++) {
        const [r, g, b, a] = pixelAt(img, x, y);
        if (!isBorder(r, g, b, a)) return false;
      }
      return true;
    };
    const colAllBorder = (x, y0, y1) => {
      for (let y = y0; y <= y1; y++) {
        const [r, g, b, a] = pixelAt(img, x, y);
        if (!isBorder(r, g, b, a)) return false;
      }
      return true;
    };
    let top = 0;
    while (top < h && rowAllBorder(top)) top++;
    let bottom = h - 1;
    while (bottom > top && rowAllBorder(bottom)) bottom--;
    let left = 0;
    while (left < w && colAllBorder(left, top, bottom)) left++;
    let right = w - 1;
    while (right > left && colAllBorder(right, top, bottom)) right--;
    return { left, top, right, bottom };
  }
  function dominantColor(img, x0, y0, x1, y1) {
    const bins = /* @__PURE__ */ new Map();
    for (let y = y0; y <= y1; y++) {
      for (let x = x0; x <= x1; x++) {
        const [r, g, b, a] = pixelAt(img, x, y);
        if (a === 0) continue;
        const key = r >> 4 << 8 | g >> 4 << 4 | b >> 4;
        let bin = bins.get(key);
        if (!bin) {
          bin = { r: 0, g: 0, b: 0, n: 0 };
          bins.set(key, bin);
        }
        bin.r += r;
        bin.g += g;
        bin.b += b;
        bin.n++;
      }
    }
    let best = null;
    for (const bin of bins.values()) {
      if (!best || bin.n > best.n) best = bin;
    }
    if (!best) return null;
    return [Math.round(best.r / best.n), Math.round(best.g / best.n), Math.round(best.b / best.n)];
  }
  function rgbDist(a, b) {
    const dr = a[0] - b[0], dg = a[1] - b[1], db = a[2] - b[2];
    return Math.sqrt(dr * dr + dg * dg + db * db);
  }
  function detectDirection(img, box) {
    const sampleAxis = (vertical, n) => {
      const colors = [];
      for (let i = 0; i < n; i++) {
        const t = i / (n - 1);
        let c;
        if (vertical) {
          const y = box.top + Math.round((box.bottom - box.top) * t);
          const pad = 2;
          c = dominantColor(img, box.left, Math.max(box.top, y - pad), box.right, Math.min(box.bottom, y + pad));
        } else {
          const x = box.left + Math.round((box.right - box.left) * t);
          const pad = 2;
          c = dominantColor(img, Math.max(box.left, x - pad), box.top, Math.min(box.right, x + pad), box.bottom);
        }
        if (c) colors.push(c);
      }
      if (colors.length < 2) return 0;
      return rgbDist(colors[0], colors[colors.length - 1]);
    };
    const vDelta = sampleAxis(true, 8);
    const hDelta = sampleAxis(false, 8);
    return vDelta >= hDelta ? { dir: "vertical", angle: 180 } : { dir: "horizontal", angle: 90 };
  }
  function extractStops(img, box, dir, n) {
    const stops = [];
    for (let i = 0; i < n; i++) {
      const t = i / (n - 1);
      let c;
      if (dir.dir === "vertical") {
        const y = box.top + Math.round((box.bottom - box.top) * t);
        const band = Math.max(2, Math.round((box.bottom - box.top) / (n * 6)));
        c = dominantColor(img, box.left, Math.max(box.top, y - band), box.right, Math.min(box.bottom, y + band));
      } else {
        const x = box.left + Math.round((box.right - box.left) * t);
        const band = Math.max(2, Math.round((box.right - box.left) / (n * 6)));
        c = dominantColor(img, Math.max(box.left, x - band), box.top, Math.min(box.right, x + band), box.bottom);
      }
      if (!c) c = [128, 128, 128];
      stops.push({ position: t, rgb: c });
    }
    return stops;
  }
  function extractGradient(path, opts = {}) {
    const img = opts.__img || loadImage(path);
    const box = opts.trim === false ? { left: 0, top: 0, right: img.width - 1, bottom: img.height - 1 } : detectInnerBox(img);
    const dirRaw = opts.direction || "auto";
    let dir;
    if (dirRaw === "auto") dir = detectDirection(img, box);
    else if (dirRaw === "vertical" || dirRaw === "180") dir = { dir: "vertical", angle: 180 };
    else if (dirRaw === "horizontal" || dirRaw === "90") dir = { dir: "horizontal", angle: 90 };
    else throw new Error(`Unknown direction "${dirRaw}". Use auto|vertical|horizontal.`);
    const nStops = opts.stops || 3;
    const stops = extractStops(img, box, dir, nStops);
    return {
      direction: dir.dir,
      angle: dir.angle,
      box,
      imageSize: { width: img.width, height: img.height },
      stops
    };
  }
  function buildFigmaPaint(result) {
    const angle = result.angle;
    const rad = (angle - 90) * Math.PI / 180;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    const tx = 0.5 - 0.5 * cos + 0.5 * sin;
    const ty = 0.5 - 0.5 * sin - 0.5 * cos;
    return {
      type: "GRADIENT_LINEAR",
      gradientTransform: [[cos, -sin, tx], [sin, cos, ty]],
      gradientStops: result.stops.map((s) => ({
        position: s.position,
        color: { r: s.rgb[0] / 255, g: s.rgb[1] / 255, b: s.rgb[2] / 255, a: 1 }
      })),
      opacity: 1,
      visible: true,
      blendMode: "NORMAL"
    };
  }
  function buildCssString(result) {
    const stops = result.stops.map((s) => {
      const hex = "#" + s.rgb.map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();
      return `${hex} ${Math.round(s.position * 100)}%`;
    });
    return `linear-gradient(${result.angle}deg, ${stops.join(", ")})`;
  }
  function rgbToHex(rgb) {
    return "#" + rgb.map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();
  }
  function brightestSpot(img, box, opts = {}) {
    const cells = opts.cells || 16;
    const xMin = opts.xMin ?? 0;
    const xMax = opts.xMax ?? 1;
    const yMin = opts.yMin ?? 0;
    const yMax = opts.yMax ?? 1;
    const W = box.right - box.left;
    const H = box.bottom - box.top;
    let best = { v: -1, fx: 0.5, fy: 0.5, color: [255, 255, 255] };
    for (let i = 0; i < cells; i++) {
      for (let j = 0; j < cells; j++) {
        const fx = xMin + (xMax - xMin) * (i / (cells - 1));
        const fy = yMin + (yMax - yMin) * (j / (cells - 1));
        const x = box.left + Math.round(W * fx);
        const y = box.top + Math.round(H * fy);
        const pad = Math.max(4, Math.round(Math.min(W, H) / 60));
        const c = dominantColor(
          img,
          Math.max(box.left, x - pad),
          Math.max(box.top, y - pad),
          Math.min(box.right, x + pad),
          Math.min(box.bottom, y + pad)
        );
        if (!c) continue;
        const v = (c[0] + c[1] + c[2]) / 3;
        if (v > best.v) best = { v, fx, fy, color: c };
      }
    }
    return best;
  }
  function reddestSpot(img, box, opts = {}) {
    const cells = opts.cells || 16;
    const xMin = opts.xMin ?? 0.2;
    const xMax = opts.xMax ?? 0.8;
    const yMin = opts.yMin ?? 0.5;
    const yMax = opts.yMax ?? 0.95;
    const W = box.right - box.left;
    const H = box.bottom - box.top;
    let best = { score: -1e9, fx: 0.5, fy: 0.85, color: [200, 80, 80] };
    for (let i = 0; i < cells; i++) {
      for (let j = 0; j < cells; j++) {
        const fx = xMin + (xMax - xMin) * (i / (cells - 1));
        const fy = yMin + (yMax - yMin) * (j / (cells - 1));
        const x = box.left + Math.round(W * fx);
        const y = box.top + Math.round(H * fy);
        const pad = Math.max(4, Math.round(Math.min(W, H) / 60));
        const c = dominantColor(
          img,
          Math.max(box.left, x - pad),
          Math.max(box.top, y - pad),
          Math.min(box.right, x + pad),
          Math.min(box.bottom, y + pad)
        );
        if (!c) continue;
        const score = c[0] - 0.5 * (c[1] + c[2]);
        if (score > best.score) best = { score, fx, fy, color: c };
      }
    }
    return best;
  }
  function averageColor(colors) {
    const sum = [0, 0, 0];
    for (const c of colors) {
      sum[0] += c[0];
      sum[1] += c[1];
      sum[2] += c[2];
    }
    return [
      Math.round(sum[0] / colors.length),
      Math.round(sum[1] / colors.length),
      Math.round(sum[2] / colors.length)
    ];
  }
  function extractMesh(path, opts = {}) {
    const img = opts.__img || loadImage(path);
    const box = opts.trim === false ? { left: 0, top: 0, right: img.width - 1, bottom: img.height - 1 } : detectInnerBox(img);
    const halfBand = Math.max(8, Math.round(Math.min(box.right - box.left, box.bottom - box.top) / 60));
    const sampleUV = (fx, fy) => {
      const x = box.left + Math.round((box.right - box.left) * fx);
      const y = box.top + Math.round((box.bottom - box.top) * fy);
      const c = dominantColor(
        img,
        Math.max(box.left, x - halfBand),
        Math.max(box.top, y - halfBand),
        Math.min(box.right, x + halfBand),
        Math.min(box.bottom, y + halfBand)
      );
      return c || [128, 128, 128];
    };
    const TL = sampleUV(0.05, 0.05);
    const TR = sampleUV(0.95, 0.05);
    const BL = sampleUV(0.05, 0.95);
    const BR = sampleUV(0.95, 0.95);
    const ML = sampleUV(0.05, 0.5);
    const MR = sampleUV(0.95, 0.5);
    const MC = sampleUV(0.5, 0.5);
    const light = brightestSpot(img, box, { yMax: 0.4 });
    const warm = reddestSpot(img, box);
    const blobs = [
      { fx: -0.05, fy: 0.45, r: 0.5, color: rgbToHex(ML) },
      { fx: 1.05, fy: 0.55, r: 0.48, color: rgbToHex(MR) },
      { fx: -0.02, fy: -0.02, r: 0.4, color: rgbToHex(TL) },
      { fx: 1.02, fy: -0.02, r: 0.4, color: rgbToHex(TR) },
      { fx: -0.02, fy: 1.02, r: 0.42, color: rgbToHex(BL) },
      { fx: 1.02, fy: 1.02, r: 0.42, color: rgbToHex(BR) },
      { fx: warm.fx, fy: warm.fy, r: 0.42, color: rgbToHex(warm.color) },
      { fx: light.fx, fy: light.fy, r: 0.5, color: rgbToHex(light.color) }
    ];
    return {
      mode: "mesh",
      base: rgbToHex(MC),
      blobs,
      blurFraction: 0.38,
      // fraction of min(W, H) — applied at apply time
      imageSize: { width: img.width, height: img.height },
      box
    };
  }
  function hexToRgb(h) {
    h = h.replace("#", "");
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }
  function normalizeHex(h) {
    return rgbToHex(hexToRgb(h));
  }
  var relLum = (rgb) => 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
  var clamp255 = (v) => Math.max(0, Math.min(255, Math.round(v)));
  var darken = (rgb, amt) => rgb.map((c) => clamp255(c * (1 - amt)));
  var lighten = (rgb, amt) => rgb.map((c) => clamp255(c + (255 - c) * amt));
  var mixRgb = (a, b, t) => a.map((c, i) => clamp255(c + (b[i] - c) * t));
  var saturate = (rgb, amt) => {
    const g = relLum(rgb);
    return rgb.map((c) => clamp255(c + (c - g) * amt));
  };
  function makeRng(seed) {
    let s = seed >>> 0;
    return () => {
      s = s + 1831565813 >>> 0;
      let t = s;
      t = Math.imul(t ^ t >>> 15, t | 1);
      t ^= t + Math.imul(t ^ t >>> 7, t | 61);
      return ((t ^ t >>> 14) >>> 0) / 4294967296;
    };
  }
  var MESH_STYLES = ["scatter", "diagonal", "bands", "drift", "spotlight", "corners"];
  function meshPositions(n, style, rng) {
    const jit = (base, amt) => base + (rng() - 0.5) * amt;
    const r = (base, amt = 0.1) => Math.max(0.5, jit(base, amt));
    const pts = [];
    if (style === "diagonal") {
      const flip = rng() > 0.5;
      for (let i = 0; i < n; i++) {
        const t = n === 1 ? 0.5 : i / (n - 1);
        const fx = (flip ? 1 - t : t) * 1.2 - 0.1;
        const fy = t * 1.2 - 0.1;
        pts.push({ fx: jit(fx, 0.2), fy: jit(fy, 0.18), r: r(0.66, 0.12) });
      }
    } else if (style === "bands") {
      const horizontal = rng() > 0.5;
      for (let i = 0; i < n; i++) {
        const t = (i + 0.5) / n * 1.2 - 0.1;
        if (horizontal) pts.push({ fx: jit(0.5, 0.5), fy: jit(t, 0.1), r: r(0.66, 0.08) });
        else pts.push({ fx: jit(t, 0.1), fy: jit(0.5, 0.5), r: r(0.66, 0.08) });
      }
    } else if (style === "drift") {
      const cx = rng() > 0.5 ? -0.1 : 1.1;
      const cy = rng() > 0.5 ? -0.1 : 1.1;
      for (let i = 0; i < n; i++) {
        const t = n === 1 ? 0 : i / (n - 1);
        const fx = cx + (1 - 2 * (cx < 0.5 ? 0 : 1)) * t * 1.05;
        const fy = cy + (1 - 2 * (cy < 0.5 ? 0 : 1)) * t * 1.05;
        pts.push({ fx: jit(fx, 0.18), fy: jit(fy, 0.18), r: r(0.7 - t * 0.12, 0.1) });
      }
    } else if (style === "spotlight") {
      pts.push({ fx: jit(0.4, 0.24), fy: jit(0.42, 0.24), r: r(0.7, 0.08) });
      const start = rng() * Math.PI * 2;
      for (let i = 1; i < n; i++) {
        const ang = start + i / Math.max(1, n - 1) * Math.PI * 2;
        pts.push({ fx: jit(0.5 + Math.cos(ang) * 0.85, 0.12), fy: jit(0.5 + Math.sin(ang) * 0.85, 0.12), r: r(0.6, 0.1) });
      }
    } else if (style === "corners") {
      const corners = [[-0.1, -0.1], [1.1, -0.1], [-0.1, 1.1], [1.1, 1.1]];
      for (let i = 0; i < n; i++) {
        if (i < 4) pts.push({ fx: jit(corners[i][0], 0.1), fy: jit(corners[i][1], 0.1), r: r(0.68, 0.08) });
        else {
          const edge = (i - 4) % 4;
          const ep = [[0.5, -0.1], [1.1, 0.5], [0.5, 1.1], [-0.1, 0.5]][edge];
          pts.push({ fx: jit(ep[0], 0.12), fy: jit(ep[1], 0.12), r: r(0.6, 0.1) });
        }
      }
    } else {
      const start = rng() * Math.PI * 2;
      for (let i = 0; i < n; i++) {
        const ang = start + i / n * Math.PI * 2 + (rng() - 0.5) * 0.35;
        const rad = 0.72 + rng() * 0.18;
        pts.push({
          fx: 0.5 + Math.cos(ang) * rad,
          fy: 0.5 + Math.sin(ang) * rad * 0.92,
          r: r(0.58, 0.1)
        });
      }
    }
    return pts;
  }
  function buildMeshFromColors(colors, opts = {}) {
    const hexes = colors.map(normalizeHex);
    if (hexes.length < 2) throw new Error("Need at least 2 colors for a mesh gradient.");
    const seed = opts.seed != null ? opts.seed >>> 0 : (Date.now() ^ Math.floor(Math.random() * 4294967295)) >>> 0;
    const rng = makeRng(seed);
    const STYLE_ALIASES = {
      aurora: "drift",
      blossom: "spotlight",
      nebula: "scatter",
      ring: "scatter",
      blob: "spotlight",
      radial: "spotlight",
      linear: "diagonal",
      stripes: "bands",
      random: "auto",
      mesh: "auto",
      "": "auto"
    };
    let style = (opts.style || "auto").toLowerCase();
    if (STYLE_ALIASES[style]) style = STYLE_ALIASES[style];
    if (style === "auto") style = MESH_STYLES[Math.floor(rng() * MESH_STYLES.length)];
    if (!MESH_STYLES.includes(style)) throw new Error(`Unknown style "${opts.style}". One of: ${MESH_STYLES.join(", ")}, auto.`);
    const order = hexes.map((_, i) => i);
    for (let i = order.length - 1; i > 0; i--) {
      const j = Math.floor(rng() * (i + 1));
      [order[i], order[j]] = [order[j], order[i]];
    }
    const positions = meshPositions(hexes.length, style, rng);
    const round = (v) => Math.round(v * 1e3) / 1e3;
    const blobs = positions.map((pos, i) => ({
      fx: round(pos.fx),
      fy: round(pos.fy),
      r: round(pos.r),
      color: hexes[order[i % order.length]]
    }));
    const rgbs = hexes.map(hexToRgb);
    const byLum = [...rgbs].sort((a, b) => relLum(a) - relLum(b));
    const darkest = byLum[0];
    const lightest = byLum[byLum.length - 1];
    const accents = opts.accents === false;
    if (!accents) {
      const dc = rng() > 0.5 ? -0.12 : 1.12;
      const dy = 1.12;
      blobs.unshift({
        fx: round(dc),
        fy: round(dy),
        r: 0.7,
        color: rgbToHex(darken(darkest, 0.35))
      });
      const gx = 0.3 + rng() * 0.4;
      const gy = 0.06 + rng() * 0.22;
      blobs.push({
        fx: round(gx),
        fy: round(gy),
        r: 0.34,
        color: rgbToHex(saturate(lighten(lightest, 0.4), 0.2)),
        blurMul: 0.62
      });
    }
    const base = opts.base ? normalizeHex(opts.base) : rgbToHex(mixRgb(averageColor(rgbs), darkest, 0.22));
    return {
      mode: "mesh",
      style,
      seed,
      base,
      blobs,
      blurFraction: opts.blur != null ? opts.blur : 0.42
    };
  }

  // tools-src/shadcn.js
  var components = {
    // ── Button ──────────────────────────────────────────────────────────
    // Base: inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium
    // Variants: default, secondary, destructive, outline, ghost, link
    // Sizes: default (h-10 px-4 py-2), sm (h-9 px-3), lg (h-11 px-8), icon (h-10 w-10)
    button: () => {
      const btn = (name, h, px, rounded, bg, fg) => `<Frame name="${name}" h={${h}} bg="${bg}" rounded={${rounded}} flex="row" justify="center" items="center" gap={8} px={${px}} py={8}><Text size={14} weight="medium" color="${fg}">Button</Text></Frame>`;
      return [
        { name: "Button / Default", jsx: btn("Button / Default", 40, 16, 6, "var:primary", "var:primary-foreground") },
        { name: "Button / Secondary", jsx: btn("Button / Secondary", 40, 16, 6, "var:secondary", "var:secondary-foreground") },
        { name: "Button / Destructive", jsx: btn("Button / Destructive", 40, 16, 6, "var:destructive", "var:destructive-foreground") },
        { name: "Button / Outline", jsx: `<Frame name="Button / Outline" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:foreground">Button</Text></Frame>` },
        { name: "Button / Ghost", jsx: `<Frame name="Button / Ghost" h={40} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:foreground">Button</Text></Frame>` },
        { name: "Button / Link", jsx: `<Frame name="Button / Link" h={40} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:primary">Button</Text></Frame>` },
        { name: "Button / Small", jsx: btn("Button / Small", 36, 12, 6, "var:primary", "var:primary-foreground") },
        { name: "Button / Large", jsx: btn("Button / Large", 44, 32, 6, "var:primary", "var:primary-foreground") },
        { name: "Button / Icon", jsx: `<Frame name="Button / Icon" w={40} h={40} bg="var:primary" rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:plus" size={16} color="var:primary-foreground" /></Frame>` }
      ];
    },
    // ── Badge ───────────────────────────────────────────────────────────
    // Base: inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold
    badge: () => [
      { name: "Badge / Default", jsx: `<Frame name="Badge / Default" bg="var:primary" rounded={9999} flex="row" items="center" px={10} py={2}><Text size={12} weight="semibold" color="var:primary-foreground">Badge</Text></Frame>` },
      { name: "Badge / Secondary", jsx: `<Frame name="Badge / Secondary" bg="var:secondary" rounded={9999} flex="row" items="center" px={10} py={2}><Text size={12} weight="semibold" color="var:secondary-foreground">Badge</Text></Frame>` },
      { name: "Badge / Destructive", jsx: `<Frame name="Badge / Destructive" bg="var:destructive" rounded={9999} flex="row" items="center" px={10} py={2}><Text size={12} weight="semibold" color="var:destructive-foreground">Badge</Text></Frame>` },
      { name: "Badge / Outline", jsx: `<Frame name="Badge / Outline" rounded={9999} flex="row" items="center" px={10} py={2} stroke="var:border" strokeWidth={1}><Text size={12} weight="semibold" color="var:foreground">Badge</Text></Frame>` }
    ],
    // ── Card ─────────────────────────────────────────────────────────────
    card: () => [
      { name: "Card", jsx: `<Frame name="Card" w={350} flex="col" bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame name="CardHeader" flex="col" gap={6} p={24} w="fill"><Text size={24} weight="semibold" color="var:card-foreground" w="fill">Card Title</Text><Text size={14} color="var:muted-foreground" w="fill">Card description goes here.</Text></Frame><Frame name="CardContent" flex="col" gap={8} px={24} pb={24} w="fill"><Text size={14} color="var:card-foreground" w="fill">Your content goes here. Add any components or text.</Text></Frame><Frame name="CardFooter" flex="row" items="center" gap={8} px={24} pb={24} w="fill" justify="start"><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Save</Text></Frame><Frame bg="var:background" stroke="var:input" strokeWidth={1} px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:foreground">Cancel</Text></Frame></Frame></Frame>` }
    ],
    // ── Input ────────────────────────────────────────────────────────────
    input: () => [
      { name: "Input / Default", jsx: `<Frame name="Input / Default" w={280} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:muted-foreground">Email</Text></Frame>` },
      { name: "Input / Filled", jsx: `<Frame name="Input / Filled" w={280} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">john@example.com</Text></Frame>` },
      { name: "Input / With Label", jsx: `<Frame name="Input / With Label" w={280} flex="col" gap={8}><Text size={14} weight="medium" color="var:foreground">Email</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:muted-foreground">m@example.com</Text></Frame></Frame>` }
    ],
    // ── Textarea ─────────────────────────────────────────────────────────
    textarea: () => [
      { name: "Textarea", jsx: `<Frame name="Textarea" w={280} h={80} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="col" p={12} pt={8}><Text size={14} color="var:muted-foreground" w="fill">Type your message here.</Text></Frame>` }
    ],
    // ── Label ────────────────────────────────────────────────────────────
    label: () => [
      { name: "Label", jsx: `<Frame name="Label" flex="row"><Text size={14} weight="medium" color="var:foreground">Label</Text></Frame>` }
    ],
    // ── Alert ────────────────────────────────────────────────────────────
    alert: () => [
      { name: "Alert / Default", jsx: `<Frame name="Alert / Default" w={400} flex="row" bg="var:background" stroke="var:border" strokeWidth={1} rounded={8} p={16} gap={12} items="start"><Icon name="lucide:info" size={16} color="var:foreground" /><Frame flex="col" gap={4} w="fill"><Text size={14} weight="medium" color="var:foreground" w="fill">Heads up!</Text><Text size={14} color="var:muted-foreground" w="fill">You can add components to your app using the CLI.</Text></Frame></Frame>` },
      { name: "Alert / Destructive", jsx: `<Frame name="Alert / Destructive" w={400} flex="row" bg="var:background" stroke="var:destructive" strokeWidth={1} rounded={8} p={16} gap={12} items="start"><Icon name="lucide:alert-circle" size={16} color="var:destructive" /><Frame flex="col" gap={4} w="fill"><Text size={14} weight="medium" color="var:destructive" w="fill">Error</Text><Text size={14} color="var:muted-foreground" w="fill">Your session has expired. Please log in again.</Text></Frame></Frame>` }
    ],
    // ── Avatar ───────────────────────────────────────────────────────────
    avatar: () => [
      { name: "Avatar / Default", jsx: `<Frame name="Avatar / Default" w={40} h={40} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Text size={16} weight="medium" color="var:muted-foreground">CN</Text></Frame>` },
      { name: "Avatar / Small", jsx: `<Frame name="Avatar / Small" w={32} h={32} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Text size={12} weight="medium" color="var:muted-foreground">CN</Text></Frame>` }
    ],
    // ── Switch ──────────────────────────────────────────────────────────
    switch: () => [
      { name: "Switch / On", jsx: `<Frame name="Switch / On" w={44} h={24} bg="var:primary" rounded={9999} flex="row" items="center" p={2} justify="end"><Frame w={20} h={20} bg="var:primary-foreground" rounded={9999} /></Frame>` },
      { name: "Switch / Off", jsx: `<Frame name="Switch / Off" w={44} h={24} bg="var:input" rounded={9999} flex="row" items="center" p={2} justify="start"><Frame w={20} h={20} bg="var:background" rounded={9999} /></Frame>` }
    ],
    // ── Separator ───────────────────────────────────────────────────────
    separator: () => [
      { name: "Separator / Horizontal", jsx: `<Frame name="Separator / Horizontal" w={280} h={1} bg="var:border" />` },
      { name: "Separator / Vertical", jsx: `<Frame name="Separator / Vertical" w={1} h={40} bg="var:border" />` }
    ],
    // ── Skeleton ────────────────────────────────────────────────────────
    skeleton: () => [
      { name: "Skeleton / Text", jsx: `<Frame name="Skeleton / Text" w={200} h={16} bg="var:muted" rounded={6} />` },
      { name: "Skeleton / Circle", jsx: `<Frame name="Skeleton / Circle" w={40} h={40} bg="var:muted" rounded={9999} />` },
      { name: "Skeleton / Card", jsx: `<Frame name="Skeleton / Card" w={350} flex="col" gap={12} p={24}><Frame flex="row" gap={16} items="center" w="fill"><Frame w={48} h={48} bg="var:muted" rounded={9999} /><Frame flex="col" gap={8} w="fill"><Frame w={200} h={16} bg="var:muted" rounded={6} /><Frame w={140} h={14} bg="var:muted" rounded={6} /></Frame></Frame><Frame w="fill" h={14} bg="var:muted" rounded={6} /><Frame w="fill" h={14} bg="var:muted" rounded={6} /><Frame w={200} h={14} bg="var:muted" rounded={6} /></Frame>` }
    ],
    // ── Progress ────────────────────────────────────────────────────────
    progress: () => [
      { name: "Progress / 60%", jsx: `<Frame name="Progress / 60%" w={280} h={8} bg="var:secondary" rounded={9999} overflow="hidden"><Frame w={168} h={8} bg="var:primary" rounded={9999} /></Frame>` },
      { name: "Progress / 30%", jsx: `<Frame name="Progress / 30%" w={280} h={8} bg="var:secondary" rounded={9999} overflow="hidden"><Frame w={84} h={8} bg="var:primary" rounded={9999} /></Frame>` }
    ],
    // ── Toggle ──────────────────────────────────────────────────────────
    toggle: () => [
      { name: "Toggle / Default", jsx: `<Frame name="Toggle / Default" w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:bold" size={16} color="var:foreground" /></Frame>` },
      { name: "Toggle / Active", jsx: `<Frame name="Toggle / Active" w={40} h={40} bg="var:accent" rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:bold" size={16} color="var:accent-foreground" /></Frame>` }
    ],
    // ── Checkbox ────────────────────────────────────────────────────────
    checkbox: () => [
      { name: "Checkbox / Unchecked", jsx: `<Frame name="Checkbox / Unchecked" w={16} h={16} bg="var:background" stroke="var:primary" strokeWidth={1} rounded={4} />` },
      { name: "Checkbox / Checked", jsx: `<Frame name="Checkbox / Checked" w={16} h={16} bg="var:primary" rounded={4} flex="row" justify="center" items="center"><Icon name="lucide:check" size={12} color="var:primary-foreground" /></Frame>` }
    ],
    // ── Tabs ─────────────────────────────────────────────────────────────
    tabs: () => [
      { name: "Tabs", jsx: `<Frame name="Tabs" w={400} flex="col" gap={8}><Frame name="TabsList" w="fill" h={40} bg="var:muted" rounded={6} flex="row" p={4} gap={4}><Frame name="Tab Active" bg="var:background" rounded={4} flex="row" justify="center" items="center" grow={1} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Text size={14} weight="medium" color="var:foreground">Account</Text></Frame><Frame name="Tab Inactive" rounded={4} flex="row" justify="center" items="center" grow={1}><Text size={14} color="var:muted-foreground">Password</Text></Frame></Frame><Frame name="TabContent" w="fill" bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} p={24} flex="col" gap={16}><Frame flex="col" gap={4} w="fill"><Text size={18} weight="semibold" color="var:card-foreground" w="fill">Account</Text><Text size={14} color="var:muted-foreground" w="fill">Make changes to your account here.</Text></Frame><Frame flex="col" gap={8} w="fill"><Text size={14} weight="medium" color="var:foreground">Name</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">Pedro Duarte</Text></Frame></Frame><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Save changes</Text></Frame></Frame></Frame>` }
    ],
    // ── Table ────────────────────────────────────────────────────────────
    table: () => [
      { name: "Table", jsx: `<Frame name="Table" w={500} flex="col" bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} overflow="hidden"><Frame name="Header" w="fill" flex="row" bg="var:muted" px={16} h={48} items="center"><Frame w={200} flex="row"><Text size={14} weight="medium" color="var:muted-foreground">Invoice</Text></Frame><Frame w={100} flex="row"><Text size={14} weight="medium" color="var:muted-foreground">Status</Text></Frame><Frame grow={1} flex="row" justify="end"><Text size={14} weight="medium" color="var:muted-foreground">Amount</Text></Frame></Frame><Frame name="Row 1" w="fill" flex="row" px={16} h={48} items="center" stroke="var:border" strokeWidth={1} strokeAlign="inside"><Frame w={200} flex="row"><Text size={14} weight="medium" color="var:card-foreground">INV001</Text></Frame><Frame w={100} flex="row"><Text size={14} color="var:card-foreground">Paid</Text></Frame><Frame grow={1} flex="row" justify="end"><Text size={14} color="var:card-foreground">$250.00</Text></Frame></Frame><Frame name="Row 2" w="fill" flex="row" px={16} h={48} items="center" stroke="var:border" strokeWidth={1} strokeAlign="inside"><Frame w={200} flex="row"><Text size={14} weight="medium" color="var:card-foreground">INV002</Text></Frame><Frame w={100} flex="row"><Text size={14} color="var:card-foreground">Pending</Text></Frame><Frame grow={1} flex="row" justify="end"><Text size={14} color="var:card-foreground">$150.00</Text></Frame></Frame><Frame name="Row 3" w="fill" flex="row" px={16} h={48} items="center"><Frame w={200} flex="row"><Text size={14} weight="medium" color="var:card-foreground">INV003</Text></Frame><Frame w={100} flex="row"><Text size={14} color="var:card-foreground">Unpaid</Text></Frame><Frame grow={1} flex="row" justify="end"><Text size={14} color="var:card-foreground">$350.00</Text></Frame></Frame></Frame>` }
    ],
    // ── Radio Group ──────────────────────────────────────────────────────
    "radio-group": () => [
      { name: "Radio / Unchecked", jsx: `<Frame name="Radio / Unchecked" w={16} h={16} stroke="var:primary" strokeWidth={1} rounded={9999} />` },
      { name: "Radio / Checked", jsx: `<Frame name="Radio / Checked" w={16} h={16} stroke="var:primary" strokeWidth={1} rounded={9999} flex="row" justify="center" items="center"><Frame w={8} h={8} bg="var:primary" rounded={9999} /></Frame>` },
      { name: "Radio Group", jsx: `<Frame name="Radio Group" flex="col" gap={12}><Frame flex="row" gap={8} items="center"><Frame w={16} h={16} stroke="var:primary" strokeWidth={1} rounded={9999} flex="row" justify="center" items="center"><Frame w={8} h={8} bg="var:primary" rounded={9999} /></Frame><Text size={14} color="var:foreground">Default</Text></Frame><Frame flex="row" gap={8} items="center"><Frame w={16} h={16} stroke="var:primary" strokeWidth={1} rounded={9999} /><Text size={14} color="var:foreground">Comfortable</Text></Frame><Frame flex="row" gap={8} items="center"><Frame w={16} h={16} stroke="var:primary" strokeWidth={1} rounded={9999} /><Text size={14} color="var:foreground">Compact</Text></Frame></Frame>` }
    ],
    // ── Select ──────────────────────────────────────────────────────────
    select: () => [
      { name: "Select / Closed", jsx: `<Frame name="Select / Closed" w={200} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12} gap={8}><Text size={14} color="var:muted-foreground">Select...</Text><Frame grow={1} /><Icon name="lucide:chevron-down" size={14} color="var:muted-foreground" /></Frame>` },
      { name: "Select / Filled", jsx: `<Frame name="Select / Filled" w={200} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12} gap={8}><Text size={14} color="var:foreground">Option A</Text><Frame grow={1} /><Icon name="lucide:chevron-down" size={14} color="var:muted-foreground" /></Frame>` },
      { name: "Select / Open", jsx: `<Frame name="Select / Open" w={200} flex="col" gap={4}><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12} gap={8}><Text size={14} color="var:foreground">Option A</Text><Frame grow={1} /><Icon name="lucide:chevron-up" size={14} color="var:muted-foreground" /></Frame><Frame w="fill" bg="var:card" stroke="var:border" strokeWidth={1} rounded={6} p={4} flex="col" shadow="0px 4px 12px rgba(0,0,0,0.1)"><Frame w="fill" h={32} bg="var:accent" rounded={4} flex="row" items="center" px={8} gap={8}><Icon name="lucide:check" size={14} color="var:accent-foreground" /><Text size={14} color="var:accent-foreground">Option A</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8} pl={30}><Text size={14} color="var:card-foreground">Option B</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8} pl={30}><Text size={14} color="var:card-foreground">Option C</Text></Frame></Frame></Frame>` }
    ],
    // ── Slider ──────────────────────────────────────────────────────────
    slider: () => [
      { name: "Slider", jsx: `<Frame name="Slider" w={280} h={20} flex="row" items="center"><Frame w={168} h={8} bg="var:primary" roundedTL={9999} roundedBL={9999} /><Frame w={112} h={8} bg="var:secondary" roundedTR={9999} roundedBR={9999} /><Frame name="Thumb" w={20} h={20} bg="var:background" stroke="var:primary" strokeWidth={2} rounded={9999} position="absolute" x={158} /></Frame>` }
    ],
    // ── Breadcrumb ──────────────────────────────────────────────────────
    breadcrumb: () => [
      { name: "Breadcrumb", jsx: `<Frame name="Breadcrumb" flex="row" gap={6} items="center"><Text size={14} color="var:muted-foreground">Home</Text><Icon name="lucide:chevron-right" size={14} color="var:muted-foreground" /><Text size={14} color="var:muted-foreground">Components</Text><Icon name="lucide:chevron-right" size={14} color="var:muted-foreground" /><Text size={14} weight="medium" color="var:foreground">Breadcrumb</Text></Frame>` }
    ],
    // ── Pagination ──────────────────────────────────────────────────────
    pagination: () => [
      { name: "Pagination", jsx: `<Frame name="Pagination" flex="row" gap={4} items="center"><Frame w={40} h={40} rounded={6} stroke="var:input" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-left" size={16} color="var:foreground" /></Frame><Frame w={40} h={40} bg="var:primary" rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">1</Text></Frame><Frame w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Text size={14} color="var:foreground">2</Text></Frame><Frame w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Text size={14} color="var:foreground">3</Text></Frame><Frame w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:ellipsis" size={16} color="var:muted-foreground" /></Frame><Frame w={40} h={40} rounded={6} stroke="var:input" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-right" size={16} color="var:foreground" /></Frame></Frame>` }
    ],
    // ── Kbd ─────────────────────────────────────────────────────────────
    kbd: () => [
      { name: "Kbd", jsx: `<Frame name="Kbd" flex="row" items="center" px={6} py={2} rounded={4} stroke="var:border" strokeWidth={1} bg="var:muted"><Text size={12} weight="medium" color="var:foreground">\u2318K</Text></Frame>` },
      { name: "Kbd / Large", jsx: `<Frame name="Kbd / Large" flex="row" items="center" gap={4}><Frame flex="row" items="center" px={6} py={2} rounded={4} stroke="var:border" strokeWidth={1} bg="var:muted"><Text size={12} weight="medium" color="var:foreground">\u2318</Text></Frame><Frame flex="row" items="center" px={6} py={2} rounded={4} stroke="var:border" strokeWidth={1} bg="var:muted"><Text size={12} weight="medium" color="var:foreground">Shift</Text></Frame><Frame flex="row" items="center" px={6} py={2} rounded={4} stroke="var:border" strokeWidth={1} bg="var:muted"><Text size={12} weight="medium" color="var:foreground">P</Text></Frame></Frame>` }
    ],
    // ── Spinner ─────────────────────────────────────────────────────────
    spinner: () => [
      { name: "Spinner / Small", jsx: `<Frame name="Spinner / Small" w={16} h={16} rounded={9999} stroke="var:primary" strokeWidth={2} />` },
      { name: "Spinner / Medium", jsx: `<Frame name="Spinner / Medium" w={24} h={24} rounded={9999} stroke="var:primary" strokeWidth={2} />` }
    ],
    // ── Tooltip ─────────────────────────────────────────────────────────
    tooltip: () => [
      { name: "Tooltip", jsx: `<Frame name="Tooltip" flex="col" items="center" gap={4}><Frame bg="var:primary" rounded={6} px={12} py={6} shadow="0px 4px 8px rgba(0,0,0,0.12)"><Text size={14} color="var:primary-foreground">Add to library</Text></Frame><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Hover me</Text></Frame></Frame>` }
    ],
    // ── Dialog ──────────────────────────────────────────────────────────
    dialog: () => [
      { name: "Dialog", jsx: `<Frame name="Dialog" w={450} flex="col" bg="var:background" stroke="var:border" strokeWidth={1} rounded={8} p={24} gap={16} shadow="0px 8px 24px rgba(0,0,0,0.15)"><Frame flex="row" items="start" w="fill"><Frame flex="col" gap={6} w="fill"><Text size={18} weight="semibold" color="var:foreground" w="fill">Edit profile</Text><Text size={14} color="var:muted-foreground" w="fill">Make changes to your profile here. Click save when you are done.</Text></Frame><Frame w={24} h={24} rounded={4} flex="row" justify="center" items="center"><Icon name="lucide:x" size={16} color="var:muted-foreground" /></Frame></Frame><Frame flex="col" gap={12} w="fill"><Frame flex="col" gap={8} w="fill"><Text size={14} weight="medium" color="var:foreground">Name</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">Pedro Duarte</Text></Frame></Frame><Frame flex="col" gap={8} w="fill"><Text size={14} weight="medium" color="var:foreground">Username</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">@peduarte</Text></Frame></Frame></Frame><Frame flex="row" justify="end" w="fill"><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Save changes</Text></Frame></Frame></Frame>` }
    ],
    // ── Dropdown Menu ───────────────────────────────────────────────────
    "dropdown-menu": () => [
      { name: "Dropdown Menu", jsx: `<Frame name="Dropdown Menu" w={200} bg="var:card" stroke="var:border" strokeWidth={1} rounded={6} p={4} flex="col" shadow="0px 4px 12px rgba(0,0,0,0.1)"><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8} bg="var:accent"><Text size={14} color="var:accent-foreground">Profile</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8}><Text size={14} color="var:card-foreground">Billing</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8}><Text size={14} color="var:card-foreground">Settings</Text></Frame><Frame w="fill" h={1} bg="var:border" /><Frame w="fill" h={32} rounded={4} flex="row" items="center" px={8}><Text size={14} color="var:card-foreground">Log out</Text></Frame></Frame>` }
    ],
    // ── Accordion ───────────────────────────────────────────────────────
    accordion: () => [
      { name: "Accordion", jsx: `<Frame name="Accordion" w={400} flex="col" bg="var:background"><Frame name="Item Open" w="fill" flex="col" stroke="var:border" strokeWidth={1} strokeAlign="inside"><Frame w="fill" flex="row" items="center" px={0} py={16}><Text size={14} weight="medium" color="var:foreground" w="fill">Is it accessible?</Text><Icon name="lucide:chevron-down" size={16} color="var:muted-foreground" /></Frame><Frame w="fill" pb={16} flex="col"><Text size={14} color="var:muted-foreground" w="fill">Yes. It adheres to the WAI-ARIA design pattern.</Text></Frame></Frame><Frame name="Item Closed" w="fill" flex="row" items="center" py={16} stroke="var:border" strokeWidth={1} strokeAlign="inside"><Text size={14} weight="medium" color="var:foreground" w="fill">Is it styled?</Text><Icon name="lucide:chevron-right" size={16} color="var:muted-foreground" /></Frame><Frame name="Item Closed 2" w="fill" flex="row" items="center" py={16} stroke="var:border" strokeWidth={1} strokeAlign="inside"><Text size={14} weight="medium" color="var:foreground" w="fill">Is it animated?</Text><Icon name="lucide:chevron-right" size={16} color="var:muted-foreground" /></Frame></Frame>` }
    ],
    // ── Navigation Menu ─────────────────────────────────────────────────
    "navigation-menu": () => [
      { name: "Navigation Menu", jsx: `<Frame name="Navigation Menu" flex="row" gap={4} items="center" bg="var:background" p={4} rounded={6}><Frame h={36} px={16} rounded={6} bg="var:accent" flex="row" items="center"><Text size={14} weight="medium" color="var:accent-foreground">Getting Started</Text></Frame><Frame h={36} px={16} rounded={6} flex="row" items="center" gap={4}><Text size={14} weight="medium" color="var:foreground">Components</Text><Icon name="lucide:chevron-down" size={12} color="var:muted-foreground" /></Frame><Frame h={36} px={16} rounded={6} flex="row" items="center"><Text size={14} weight="medium" color="var:foreground">Documentation</Text></Frame></Frame>` }
    ],
    // ── Sheet ───────────────────────────────────────────────────────────
    sheet: () => [
      { name: "Sheet", jsx: `<Frame name="Sheet" w={380} h={500} bg="var:background" stroke="var:border" strokeWidth={1} p={24} flex="col" gap={16} shadow="0px 8px 24px rgba(0,0,0,0.15)"><Frame flex="row" items="start" w="fill"><Frame flex="col" gap={6} w="fill"><Text size={18} weight="semibold" color="var:foreground" w="fill">Edit profile</Text><Text size={14} color="var:muted-foreground" w="fill">Make changes to your profile here.</Text></Frame><Frame w={24} h={24} rounded={4} flex="row" justify="center" items="center"><Icon name="lucide:x" size={16} color="var:muted-foreground" /></Frame></Frame><Frame flex="col" gap={12} w="fill"><Frame flex="col" gap={8} w="fill"><Text size={14} weight="medium" color="var:foreground">Name</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">Pedro Duarte</Text></Frame></Frame><Frame flex="col" gap={8} w="fill"><Text size={14} weight="medium" color="var:foreground">Username</Text><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12}><Text size={14} color="var:foreground">@peduarte</Text></Frame></Frame></Frame><Frame grow={1} /><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Save changes</Text></Frame></Frame>` }
    ],
    // ── Hover Card ──────────────────────────────────────────────────────
    "hover-card": () => [
      { name: "Hover Card", jsx: `<Frame name="Hover Card" w={320} bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} p={16} flex="row" gap={16} shadow="0px 4px 12px rgba(0,0,0,0.1)"><Frame w={40} h={40} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Text size={16} weight="medium" color="var:muted-foreground">@</Text></Frame><Frame flex="col" gap={8} w="fill"><Frame flex="col" gap={2} w="fill"><Text size={14} weight="semibold" color="var:card-foreground" w="fill">@nextjs</Text><Text size={14} color="var:muted-foreground" w="fill">The React Framework, created and maintained by @vercel.</Text></Frame><Text size={12} color="var:muted-foreground">Joined December 2021</Text></Frame></Frame>` }
    ],
    // ── Popover ─────────────────────────────────────────────────────────
    popover: () => [
      { name: "Popover", jsx: `<Frame name="Popover" w={260} bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} p={16} flex="col" gap={12} shadow="0px 4px 12px rgba(0,0,0,0.1)"><Frame flex="col" gap={4} w="fill"><Text size={14} weight="semibold" color="var:card-foreground" w="fill">Dimensions</Text><Text size={14} color="var:muted-foreground" w="fill">Set the dimensions for the layer.</Text></Frame><Frame flex="col" gap={8} w="fill"><Frame flex="row" gap={8} items="center" w="fill"><Text size={14} color="var:foreground" w={80}>Width</Text><Frame grow={1} h={32} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={10}><Text size={13} color="var:foreground">100%</Text></Frame></Frame><Frame flex="row" gap={8} items="center" w="fill"><Text size={14} color="var:foreground" w={80}>Height</Text><Frame grow={1} h={32} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={10}><Text size={13} color="var:foreground">25px</Text></Frame></Frame></Frame></Frame>` }
    ],
    // ── Alert Dialog ────────────────────────────────────────────────────
    "alert-dialog": () => [
      { name: "Alert Dialog", jsx: `<Frame name="Alert Dialog" w={450} flex="col" bg="var:background" stroke="var:border" strokeWidth={1} rounded={8} p={24} gap={16} shadow="0px 8px 24px rgba(0,0,0,0.15)"><Frame flex="col" gap={6} w="fill"><Text size={18} weight="semibold" color="var:foreground" w="fill">Are you absolutely sure?</Text><Text size={14} color="var:muted-foreground" w="fill">This action cannot be undone. This will permanently delete your account and remove your data from our servers.</Text></Frame><Frame flex="row" justify="end" gap={8} w="fill"><Frame bg="var:background" stroke="var:input" strokeWidth={1} px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:foreground">Cancel</Text></Frame><Frame bg="var:destructive" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:destructive-foreground">Continue</Text></Frame></Frame></Frame>` }
    ],
    // ── Drawer ──────────────────────────────────────────────────────────
    drawer: () => [
      { name: "Drawer", jsx: `<Frame name="Drawer" w={400} flex="col" bg="var:background" stroke="var:border" strokeWidth={1} rounded={12} p={24} gap={16} shadow="0px -4px 12px rgba(0,0,0,0.1)"><Frame w={32} h={4} bg="var:muted" rounded={9999} /><Frame flex="col" gap={6} w="fill"><Text size={18} weight="semibold" color="var:foreground" w="fill">Move Goal</Text><Text size={14} color="var:muted-foreground" w="fill">Set your daily activity goal.</Text></Frame><Frame flex="row" justify="center" items="center" gap={32} w="fill" py={16}><Frame w={40} h={40} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Icon name="lucide:minus" size={16} color="var:foreground" /></Frame><Frame flex="col" items="center" gap={2}><Text size={32} weight="bold" color="var:foreground">350</Text><Text size={12} color="var:muted-foreground">CALORIES/DAY</Text></Frame><Frame w={40} h={40} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Icon name="lucide:plus" size={16} color="var:foreground" /></Frame></Frame><Frame flex="col" gap={8} w="fill"><Frame bg="var:primary" w="fill" px={16} py={10} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Submit</Text></Frame><Frame bg="var:background" stroke="var:input" strokeWidth={1} w="fill" px={16} py={10} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:foreground">Cancel</Text></Frame></Frame></Frame>` }
    ],
    // ── Toggle Group ────────────────────────────────────────────────────
    "toggle-group": () => [
      { name: "Toggle Group", jsx: `<Frame name="Toggle Group" flex="row" gap={4} items="center"><Frame w={40} h={40} bg="var:accent" rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:bold" size={16} color="var:accent-foreground" /></Frame><Frame w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:italic" size={16} color="var:foreground" /></Frame><Frame w={40} h={40} rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:underline" size={16} color="var:foreground" /></Frame></Frame>` },
      { name: "Toggle Group / Outline", jsx: `<Frame name="Toggle Group / Outline" flex="row" stroke="var:input" strokeWidth={1} rounded={6} overflow="hidden"><Frame w={40} h={40} bg="var:accent" flex="row" justify="center" items="center"><Icon name="lucide:align-left" size={16} color="var:accent-foreground" /></Frame><Frame w={1} h={40} bg="var:border" /><Frame w={40} h={40} flex="row" justify="center" items="center"><Icon name="lucide:align-center" size={16} color="var:foreground" /></Frame><Frame w={1} h={40} bg="var:border" /><Frame w={40} h={40} flex="row" justify="center" items="center"><Icon name="lucide:align-right" size={16} color="var:foreground" /></Frame></Frame>` }
    ],
    // ── Input OTP ───────────────────────────────────────────────────────
    "input-otp": () => [
      { name: "Input OTP", jsx: `<Frame name="Input OTP" flex="row" gap={8} items="center"><Frame w={40} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center"><Text size={16} weight="medium" color="var:foreground">1</Text></Frame><Frame w={40} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center"><Text size={16} weight="medium" color="var:foreground">2</Text></Frame><Frame w={40} h={40} bg="var:background" stroke="var:primary" strokeWidth={2} rounded={6} flex="row" justify="center" items="center"><Text size={16} weight="medium" color="var:foreground">3</Text></Frame><Frame w={8} h={1} bg="var:muted-foreground" /><Frame w={40} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center" /><Frame w={40} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center" /><Frame w={40} h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center" /></Frame>` }
    ],
    // ── Sonner / Toast ──────────────────────────────────────────────────
    sonner: () => [
      { name: "Toast / Default", jsx: `<Frame name="Toast / Default" w={360} bg="var:background" stroke="var:border" strokeWidth={1} rounded={8} p={16} flex="row" gap={12} items="start" shadow="0px 4px 12px rgba(0,0,0,0.1)"><Icon name="lucide:check-circle" size={20} color="var:primary" /><Frame flex="col" gap={2} w="fill"><Text size={14} weight="semibold" color="var:foreground" w="fill">Event has been created</Text><Text size={13} color="var:muted-foreground" w="fill">Sunday, December 03, 2023 at 9:00 AM</Text></Frame><Frame flex="row" items="center" gap={4} px={10} py={6} rounded={6} bg="var:secondary"><Text size={13} weight="medium" color="var:secondary-foreground">Undo</Text></Frame></Frame>` },
      { name: "Toast / Destructive", jsx: `<Frame name="Toast / Destructive" w={360} bg="var:destructive" rounded={8} p={16} flex="row" gap={12} items="start" shadow="0px 4px 12px rgba(0,0,0,0.1)"><Icon name="lucide:alert-circle" size={20} color="var:destructive-foreground" /><Frame flex="col" gap={2} w="fill"><Text size={14} weight="semibold" color="var:destructive-foreground" w="fill">Uh oh! Something went wrong.</Text><Text size={13} color="var:destructive-foreground" w="fill">There was a problem with your request.</Text></Frame></Frame>` }
    ],
    // ── Command ─────────────────────────────────────────────────────────
    command: () => [
      { name: "Command", jsx: `<Frame name="Command" w={460} bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} flex="col" overflow="hidden" shadow="0px 8px 24px rgba(0,0,0,0.15)"><Frame w="fill" flex="row" items="center" gap={8} px={16} py={12} stroke="var:border" strokeWidth={1} strokeAlign="inside"><Icon name="lucide:search" size={16} color="var:muted-foreground" /><Text size={14} color="var:muted-foreground" w="fill">Type a command or search...</Text><Frame flex="row" items="center" px={6} py={2} rounded={4} stroke="var:border" strokeWidth={1} bg="var:muted"><Text size={12} weight="medium" color="var:muted-foreground">\u2318K</Text></Frame></Frame><Frame w="fill" flex="col" p={8}><Text size={12} weight="medium" color="var:muted-foreground" px={8} py={6}>Suggestions</Text><Frame w="fill" h={36} rounded={6} bg="var:accent" flex="row" items="center" gap={10} px={10}><Icon name="lucide:calendar" size={16} color="var:accent-foreground" /><Text size={14} color="var:accent-foreground">Calendar</Text></Frame><Frame w="fill" h={36} rounded={6} flex="row" items="center" gap={10} px={10}><Icon name="lucide:smile" size={16} color="var:card-foreground" /><Text size={14} color="var:card-foreground">Search Emoji</Text></Frame><Frame w="fill" h={36} rounded={6} flex="row" items="center" gap={10} px={10}><Icon name="lucide:calculator" size={16} color="var:card-foreground" /><Text size={14} color="var:card-foreground">Calculator</Text></Frame></Frame></Frame>` }
    ],
    // ── Combobox ────────────────────────────────────────────────────────
    combobox: () => [
      { name: "Combobox", jsx: `<Frame name="Combobox" w={260} flex="col" gap={4}><Frame w="fill" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" items="center" px={12} gap={8}><Text size={14} color="var:foreground" w="fill">Select framework...</Text><Icon name="lucide:chevrons-up-down" size={14} color="var:muted-foreground" /></Frame><Frame w="fill" bg="var:card" stroke="var:border" strokeWidth={1} rounded={6} p={4} flex="col" shadow="0px 4px 12px rgba(0,0,0,0.1)"><Frame w="fill" flex="row" items="center" gap={8} px={8} py={8} stroke="var:border" strokeWidth={1} strokeAlign="inside"><Icon name="lucide:search" size={14} color="var:muted-foreground" /><Text size={14} color="var:muted-foreground" w="fill">Search framework...</Text></Frame><Frame w="fill" h={32} rounded={4} bg="var:accent" flex="row" items="center" gap={8} px={8}><Icon name="lucide:check" size={14} color="var:accent-foreground" /><Text size={14} color="var:accent-foreground">Next.js</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" gap={8} px={8} pl={28}><Text size={14} color="var:card-foreground">Remix</Text></Frame><Frame w="fill" h={32} rounded={4} flex="row" items="center" gap={8} px={8} pl={28}><Text size={14} color="var:card-foreground">Astro</Text></Frame></Frame></Frame>` }
    ],
    // ── Calendar ────────────────────────────────────────────────────────
    calendar: () => [
      { name: "Calendar", jsx: `<Frame name="Calendar" w={280} bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} p={12} flex="col" gap={12} shadow="0px 4px 12px rgba(0,0,0,0.08)"><Frame w="fill" flex="row" items="center" justify="between"><Frame w={28} h={28} rounded={6} stroke="var:border" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-left" size={14} color="var:foreground" /></Frame><Text size={14} weight="medium" color="var:foreground">December 2023</Text><Frame w={28} h={28} rounded={6} stroke="var:border" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-right" size={14} color="var:foreground" /></Frame></Frame><Frame w="fill" flex="row" gap={0}><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Mo</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Tu</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">We</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Th</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Fr</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Sa</Text></Frame><Frame grow={1} h={28} flex="row" justify="center" items="center"><Text size={12} color="var:muted-foreground">Su</Text></Frame></Frame><Frame w="fill" flex="row"><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:muted-foreground">27</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:muted-foreground">28</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">29</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">30</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">1</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">2</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">3</Text></Frame></Frame><Frame w="fill" flex="row"><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">4</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">5</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">6</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">7</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">8</Text></Frame><Frame grow={1} h={32} bg="var:primary" rounded={6} flex="row" justify="center" items="center"><Text size={13} weight="medium" color="var:primary-foreground">9</Text></Frame><Frame grow={1} h={32} flex="row" justify="center" items="center"><Text size={13} color="var:foreground">10</Text></Frame></Frame></Frame>` }
    ],
    // ── Carousel ────────────────────────────────────────────────────────
    carousel: () => [
      { name: "Carousel", jsx: `<Frame name="Carousel" w={500} flex="col" gap={16}><Frame w="fill" flex="row" gap={16} items="center"><Frame w={36} h={36} rounded={9999} stroke="var:input" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-left" size={16} color="var:foreground" /></Frame><Frame grow={1} h={240} bg="var:card" stroke="var:border" strokeWidth={1} rounded={8} flex="row" justify="center" items="center"><Text size={48} weight="semibold" color="var:card-foreground">1</Text></Frame><Frame w={36} h={36} rounded={9999} stroke="var:input" strokeWidth={1} flex="row" justify="center" items="center"><Icon name="lucide:chevron-right" size={16} color="var:foreground" /></Frame></Frame><Frame w="fill" flex="row" gap={8} justify="center"><Frame w={8} h={8} bg="var:primary" rounded={9999} /><Frame w={8} h={8} bg="var:muted" rounded={9999} /><Frame w={8} h={8} bg="var:muted" rounded={9999} /><Frame w={8} h={8} bg="var:muted" rounded={9999} /><Frame w={8} h={8} bg="var:muted" rounded={9999} /></Frame></Frame>` }
    ],
    // ── Menubar ─────────────────────────────────────────────────────────
    menubar: () => [
      { name: "Menubar", jsx: `<Frame name="Menubar" flex="row" gap={4} items="center" bg="var:background" stroke="var:input" strokeWidth={1} p={4} rounded={6}><Frame h={28} px={12} rounded={4} bg="var:accent" flex="row" items="center"><Text size={13} weight="medium" color="var:accent-foreground">File</Text></Frame><Frame h={28} px={12} rounded={4} flex="row" items="center"><Text size={13} weight="medium" color="var:foreground">Edit</Text></Frame><Frame h={28} px={12} rounded={4} flex="row" items="center"><Text size={13} weight="medium" color="var:foreground">View</Text></Frame><Frame h={28} px={12} rounded={4} flex="row" items="center"><Text size={13} weight="medium" color="var:foreground">Profiles</Text></Frame></Frame>` }
    ],
    // ── Sidebar ─────────────────────────────────────────────────────────
    sidebar: () => [
      { name: "Sidebar", jsx: `<Frame name="Sidebar" w={260} h={520} bg="var:card" stroke="var:border" strokeWidth={1} flex="col" p={12} gap={4}><Frame w="fill" flex="row" items="center" gap={10} p={8} rounded={6}><Frame w={28} h={28} bg="var:primary" rounded={6} flex="row" justify="center" items="center"><Icon name="lucide:command" size={14} color="var:primary-foreground" /></Frame><Frame flex="col" w="fill"><Text size={13} weight="semibold" color="var:card-foreground" w="fill">Acme Inc</Text><Text size={11} color="var:muted-foreground" w="fill">Enterprise</Text></Frame></Frame><Frame w="fill" h={1} bg="var:border" /><Text size={11} weight="medium" color="var:muted-foreground" px={8} py={6}>PLATFORM</Text><Frame w="fill" h={32} bg="var:accent" rounded={6} flex="row" items="center" gap={10} px={8}><Icon name="lucide:layout-dashboard" size={14} color="var:accent-foreground" /><Text size={13} color="var:accent-foreground" w="fill">Dashboard</Text></Frame><Frame w="fill" h={32} rounded={6} flex="row" items="center" gap={10} px={8}><Icon name="lucide:inbox" size={14} color="var:card-foreground" /><Text size={13} color="var:card-foreground" w="fill">Inbox</Text></Frame><Frame w="fill" h={32} rounded={6} flex="row" items="center" gap={10} px={8}><Icon name="lucide:calendar" size={14} color="var:card-foreground" /><Text size={13} color="var:card-foreground" w="fill">Calendar</Text></Frame><Frame w="fill" h={32} rounded={6} flex="row" items="center" gap={10} px={8}><Icon name="lucide:settings" size={14} color="var:card-foreground" /><Text size={13} color="var:card-foreground" w="fill">Settings</Text></Frame></Frame>` }
    ]
  };
  var variety = {
    // Distinct button styles: default, secondary, outline, ghost, destructive, link.
    // Asking for "4 buttons" gives a useful style set, not 4 identical primaries.
    // All named just "Button" (independent frames), bound to var: tokens.
    button: () => [
      { name: "Button Default", jsx: `<Frame name="Button Default" h={40} bg="var:primary" rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:primary-foreground">Button</Text></Frame>` },
      { name: "Button Secondary", jsx: `<Frame name="Button Secondary" h={40} bg="var:secondary" rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:secondary-foreground">Button</Text></Frame>` },
      { name: "Button Outline", jsx: `<Frame name="Button Outline" h={40} bg="var:background" stroke="var:input" strokeWidth={1} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:foreground">Button</Text></Frame>` },
      { name: "Button Ghost", jsx: `<Frame name="Button Ghost" h={40} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:foreground">Button</Text></Frame>` },
      { name: "Button Destructive", jsx: `<Frame name="Button Destructive" h={40} bg="var:destructive" rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:destructive-foreground">Button</Text></Frame>` },
      { name: "Button Link", jsx: `<Frame name="Button Link" h={40} rounded={6} flex="row" justify="center" items="center" gap={8} px={16} py={8}><Text size={14} weight="medium" color="var:primary">Button</Text></Frame>` }
    ],
    // Distinct card archetypes: simple, stat, profile, media, notification, pricing.
    card: () => [
      { name: "Card Simple", jsx: `<Frame name="Card Simple" w={340} flex="col" bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame name="CardHeader" flex="col" gap={6} p={24} w="fill"><Text size={20} weight="semibold" color="var:card-foreground" w="fill">Card Title</Text><Text size={14} color="var:muted-foreground" w="fill">Card description goes here.</Text></Frame><Frame name="CardContent" flex="col" gap={8} px={24} pb={24} w="fill"><Text size={14} color="var:card-foreground" w="fill">Your content goes here. Add any components or text.</Text></Frame><Frame name="CardFooter" flex="row" items="center" gap={8} px={24} pb={24} w="fill" justify="start"><Frame bg="var:primary" px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Save</Text></Frame><Frame bg="var:background" stroke="var:input" strokeWidth={1} px={16} py={8} rounded={6} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:foreground">Cancel</Text></Frame></Frame></Frame>` },
      { name: "Card Stat", jsx: `<Frame name="Card Stat" w={340} flex="col" gap={8} bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} p={24} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame flex="row" items="center" justify="between" w="fill"><Text size={14} weight="medium" color="var:muted-foreground" w="fill">Total Revenue</Text><Icon name="lucide:trending-up" size={16} color="var:primary" /></Frame><Text size={32} weight="bold" color="var:card-foreground" w="fill">$45,231.89</Text><Text size={13} color="var:muted-foreground" w="fill">+20.1% from last month</Text></Frame>` },
      { name: "Card Profile", jsx: `<Frame name="Card Profile" w={340} flex="col" gap={16} bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} p={24} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame flex="row" items="center" gap={12} w="fill"><Frame w={48} h={48} bg="var:muted" rounded={9999} flex="row" justify="center" items="center"><Text size={18} weight="semibold" color="var:muted-foreground">JD</Text></Frame><Frame flex="col" gap={2} w="fill"><Text size={16} weight="semibold" color="var:card-foreground" w="fill">Jane Doe</Text><Text size={13} color="var:muted-foreground" w="fill">Product Designer</Text></Frame></Frame><Text size={14} color="var:card-foreground" w="fill">Building thoughtful interfaces and design systems for modern teams.</Text><Frame bg="var:primary" w="fill" px={16} py={10} rounded={8} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Follow</Text></Frame></Frame>` },
      { name: "Card Media", jsx: `<Frame name="Card Media" w={340} flex="col" bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} overflow="hidden" shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame w="fill" h={160} bg="var:muted" flex="row" justify="center" items="center"><Icon name="lucide:image" size={32} color="var:muted-foreground" /></Frame><Frame flex="col" gap={6} p={20} w="fill"><Text size={18} weight="semibold" color="var:card-foreground" w="fill">Mountain Escape</Text><Text size={14} color="var:muted-foreground" w="fill">A weekend retreat in the alps with panoramic views.</Text></Frame></Frame>` },
      { name: "Card Notification", jsx: `<Frame name="Card Notification" w={340} flex="row" gap={12} items="start" bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} p={20} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame w={36} h={36} bg="var:accent" rounded={9999} flex="row" justify="center" items="center"><Icon name="lucide:bell" size={16} color="var:accent-foreground" /></Frame><Frame flex="col" gap={4} w="fill"><Text size={14} weight="semibold" color="var:card-foreground" w="fill">New comment on your post</Text><Text size={13} color="var:muted-foreground" w="fill">Alex replied: "This looks great, ship it!"</Text><Text size={12} color="var:muted-foreground" w="fill">2 minutes ago</Text></Frame></Frame>` },
      { name: "Card Pricing", jsx: `<Frame name="Card Pricing" w={340} flex="col" gap={16} bg="var:card" stroke="var:border" strokeWidth={1} rounded={12} p={24} shadow="0px 1px 2px rgba(0,0,0,0.05)"><Frame flex="col" gap={4} w="fill"><Text size={14} weight="medium" color="var:muted-foreground" w="fill">Pro</Text><Frame flex="row" items="end" gap={4}><Text size={36} weight="bold" color="var:card-foreground">$29</Text><Text size={14} color="var:muted-foreground">/month</Text></Frame></Frame><Frame flex="col" gap={8} w="fill"><Frame flex="row" gap={8} items="center" w="fill"><Icon name="lucide:check" size={16} color="var:primary" /><Text size={14} color="var:card-foreground" w="fill">Unlimited projects</Text></Frame><Frame flex="row" gap={8} items="center" w="fill"><Icon name="lucide:check" size={16} color="var:primary" /><Text size={14} color="var:card-foreground" w="fill">Priority support</Text></Frame><Frame flex="row" gap={8} items="center" w="fill"><Icon name="lucide:check" size={16} color="var:primary" /><Text size={14} color="var:card-foreground" w="fill">Advanced analytics</Text></Frame></Frame><Frame bg="var:primary" w="fill" px={16} py={10} rounded={8} flex="row" justify="center" items="center"><Text size={14} weight="medium" color="var:primary-foreground">Get started</Text></Frame></Frame>` }
    ]
  };
  var VISUAL_COMPONENTS = Object.keys(components);
  var INTERACTIVE_ONLY = [
    "context-menu",
    "collapsible",
    "scroll-area",
    "form",
    "resizable"
  ];
  function listComponents() {
    return { available: VISUAL_COMPONENTS, interactive: INTERACTIVE_ONLY };
  }
  function getComponent(name) {
    const fn = components[name];
    if (!fn) return null;
    return fn();
  }
  function getVariety(name, count) {
    const fn = variety[name];
    if (!fn) return null;
    const pool = fn();
    if (!pool.length) return null;
    const out = [];
    for (let i = 0; i < count; i++) out.push(pool[i % pool.length]);
    return out;
  }
  function getAllComponents() {
    const all = [];
    for (const name of VISUAL_COMPONENTS) {
      all.push(...components[name]());
    }
    return all;
  }

  // tools-src/entry.mjs
  globalThis.__tools = {
    // shadcn/ui: list available components.
    shadcnList() {
      return JSON.stringify(listComponents());
    },
    // shadcn/ui: select component JSX to render. argsJson: { names, all, count }.
    // Mirrors the selection logic in commands/variants.js `shadcn add`.
    shadcnAdd(argsJson) {
      const { names, all, count } = JSON.parse(argsJson);
      let items = [];
      if (all) {
        items = getAllComponents();
      } else {
        const userPassedCount = count !== 1;
        for (const name of names) {
          const comp = getComponent(name);
          if (!comp) return JSON.stringify({ error: "Unknown component: " + name });
          if (userPassedCount) {
            const varietySet = getVariety(name, count);
            if (varietySet) {
              items.push(...varietySet);
            } else {
              const base = comp[0];
              const cleanName = base.name.split(" / ")[0];
              const cleanItem = { ...base, name: cleanName, jsx: base.jsx.replace(/name="[^"]*"/, 'name="' + cleanName + '"') };
              for (let i = 0; i < count; i++) items.push(cleanItem);
            }
          } else {
            items.push(...comp);
          }
        }
      }
      return JSON.stringify({ items });
    },
    // Linear gradient from injected pixels. argsJson: { img, direction, stops, trim }
    gradientExtract(argsJson) {
      const a = JSON.parse(argsJson);
      const result = extractGradient("__INJECTED__", { __img: a.img, direction: a.direction, stops: a.stops, trim: a.trim });
      return JSON.stringify({ result, css: buildCssString(result), paint: buildFigmaPaint(result) });
    },
    // Mesh recipe from injected pixels. argsJson: { img, trim, blur }
    meshExtract(argsJson) {
      const a = JSON.parse(argsJson);
      const recipe = extractMesh("__INJECTED__", { __img: a.img, trim: a.trim });
      if (a.blur != null) recipe.blurFraction = a.blur;
      return JSON.stringify(recipe);
    },
    // Mesh recipe from a color palette (no image). argsJson: { colors, base, blur, style, seed }
    meshFromColors(argsJson) {
      const a = JSON.parse(argsJson);
      const recipe = buildMeshFromColors(a.colors, { base: a.base, blur: a.blur, style: a.style, seed: a.seed });
      return JSON.stringify(recipe);
    }
  };
})();
