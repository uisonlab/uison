// uison.js — glue between the browser and a uison WASM app. No dependencies.
//
// Two ways to start (drawing and input code is shared):
// • Rust (wasm32-unknown-unknown): the file is the page module (index.html), loads app.wasm; `main()`
//   registers the app (`uison::run`), imports come from the `uison` module (uison_js_* functions).
// • C/C++ via Emscripten: the whole file is the body of uison_js_boot (EM_JS inside libuison.a),
//   called by uison_run from main; the uison_js_* imports are EM_JS too, they call globalThis.uisonJs.
// Then JS reports events (uison_pointer, uison_key, uison_wheel, uison_tick, uison_resize,
// uison_appearance), and the app draws on the canvas through the imports below.
//
// URL parameters: ?stats=1 — timings to the console, ?flash=1 — highlight
// redraws, ?test=1 — logs to the server (tools/serve.py), ?script=… — drive.js scenario.

const params = new URLSearchParams(location.search);
const canvas = document.getElementById("uison") || newCanvas();
const ctx = canvas.getContext("2d");
const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const dec = new TextDecoder();
const enc = new TextEncoder();

let wasm = null;
const str = (p, n) => dec.decode(new Uint8Array(wasm.memory.buffer, p, n));

// Colors: u32 0xRRGGBBAA → CSS string (cached — a frame has few colors).
const colors = new Map();
const col = (c) => {
  let s = colors.get(c);
  if (!s) {
    s = `rgba(${c >>> 24},${(c >>> 16) & 255},${(c >>> 8) & 255},${(c & 255) / 255})`;
    colors.set(c, s);
  }
  return s;
};

// Fonts are registered once; ctx.font is changed only when the font differs.
const fonts = [];
let curFont = -1;
const setFont = (id) => {
  if (id !== curFont) {
    ctx.font = fonts[id];
    curFont = id;
  }
};

const roundRect = (x, y, w, h, r) => {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, Math.min(r, w / 2, h / 2));
};

// Images: source → { bmp } (loaded) or null (loading).
const images = new Map();
function loadImage(src, blob) {
  images.set(src, null);
  (blob ? Promise.resolve(blob) : fetch(src).then((r) => (r.ok ? r.blob() : Promise.reject(new Error(r.status)))))
    .then((b) => createImageBitmap(b))
    .then(
      (bmp) => {
        images.set(src, { bmp });
        wasm.uison_image_loaded();
      },
      (e) => console.error(`uison: image ${src}:`, e.message),
    );
}


const cursors = ["default", "pointer", "text"];

// Full-window canvas if the page has none (Emscripten's default page).
function newCanvas() {
  const c = document.createElement("canvas");
  c.id = "uison";
  c.tabIndex = 0;
  const style = document.createElement("style");
  style.textContent = `html, body { margin: 0; height: 100%; overflow: hidden; background: #f5f5f7; }
    @media (prefers-color-scheme: dark) { html, body { background: #1c1c1e; } }
    #uison { display: block; width: 100vw; height: 100vh; outline: none; touch-action: none; }`;
  document.head.append(style);
  document.body.replaceChildren(c);
  return c;
}

const api = {
  now: () => performance.now(),
  log: (p, n, level) => (level === 2 ? console.error : console.log)(str(p, n)),
  set_title: (p, n) => (document.title = str(p, n)),
  set_cursor: (k) => (canvas.style.cursor = cursors[k] || "default"),
  request_tick: (ms) => setTimeout(() => wasm.uison_tick(), ms),
  request_frame: () => requestAnimationFrame(() => wasm.uison_frame()),
wake: () => setTimeout(() => wasm.uison_wake(), 0),
  mods: () => curMods,
  clipboard_write: (p, n) =>
    navigator.clipboard?.writeText(str(p, n)).catch((e) => console.error("uison: clipboard:", e.message)),
  // The app doesn't handle ⌘V / Ctrl+V — the browser sends a paste event itself (no permission
  // prompt); readText only if there was none (the "Paste" menu item).
  clipboard_read: () => {
    pasteWanted = true;
    setTimeout(() => {
      if (!pasteWanted) return;
      pasteWanted = false;
      navigator.clipboard?.readText().then(paste, (e) => console.error("uison: clipboard:", e.message));
    }, 0);
  },
  open_url: (p, n) => (window.open(str(p, n), "_blank", "noopener"), 1),

  clip_push: (x, y, w, h) => {
    ctx.save();
    ctx.beginPath();
    ctx.rect(x, y, w, h);
    ctx.clip();
  },
  clip_pop: () => {
    ctx.restore();
    curFont = -1; // restore brought back the previous ctx.font
  },
  fill_rect: (x, y, w, h, r, c) => {
    ctx.fillStyle = col(c);
    if (r > 0) {
      roundRect(x, y, w, h, r);
      ctx.fill();
    } else ctx.fillRect(x, y, w, h);
  },
  stroke_rect: (x, y, w, h, r, lw, c) => {
    ctx.strokeStyle = col(c);
    ctx.lineWidth = lw;
    if (r > 0) {
      roundRect(x, y, w, h, r);
      ctx.stroke();
    } else ctx.strokeRect(x, y, w, h);
  },
  fill_circle: (x, y, r, c) => {
    ctx.fillStyle = col(c);
    ctx.beginPath();
    ctx.arc(x, y, r, 0, 2 * Math.PI);
    ctx.fill();
  },
  stroke_circle: (x, y, r, lw, c) => {
    ctx.strokeStyle = col(c);
    ctx.lineWidth = lw;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, 2 * Math.PI);
    ctx.stroke();
  },
  poly: (p, n, fill, stroke, lw) => {
    const xy = new Float32Array(wasm.memory.buffer, p, 2 * n);
    ctx.beginPath();
    ctx.moveTo(xy[0], xy[1]);
    for (let i = 1; i < n; i++) ctx.lineTo(xy[2 * i], xy[2 * i + 1]);
    ctx.closePath();
    if (fill) {
      ctx.fillStyle = col(fill);
      ctx.fill();
    }
    if (stroke) {
      ctx.strokeStyle = col(stroke);
      ctx.lineWidth = lw;
      ctx.stroke();
    }
  },
  line: (x1, y1, x2, y2, lw, c, round) => {
    ctx.strokeStyle = col(c);
    ctx.lineWidth = lw;
    ctx.lineCap = round ? "round" : "butt";
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  },
  text: (p, n, x, y, f, c) => {
    const s = str(p, n);
    if (window.uison.skipFill) return; // measurement: string decoding only
    setFont(f);
    ctx.fillStyle = col(c);
    ctx.fillText(s, x, y);
  },
  font: (p, n) => fonts.push(str(p, n)) - 1,
  image: (p, n, x, y, w, h, r, fit) => {
    const src = str(p, n);
    const im = images.get(src);
    if (im === undefined) {
      if (src.startsWith("mem:")) return 0; // Rust will send the bytes (image_data)
      loadImage(src);
      return 1;
    }
    if (!im) return 1;
    const { bmp } = im;
    const iw = bmp.width, ih = bmp.height;
    let [sx, sy, sw, sh, dx, dy, dw, dh] = [0, 0, iw, ih, x, y, w, h];
    if (fit === 0) {
      const k = Math.max(w / iw, h / ih);
      [sw, sh] = [w / k, h / k];
      [sx, sy] = [(iw - sw) / 2, (ih - sh) / 2];
    } else if (fit === 1) {
      const k = Math.min(w / iw, h / ih);
      [dw, dh] = [iw * k, ih * k];
      [dx, dy] = [x + (w - dw) / 2, y + (h - dh) / 2];
    }
    ctx.save();
    roundRect(dx, dy, dw, dh, r);
    ctx.clip();
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(bmp, sx, sy, sw, sh, dx, dy, dw, dh);
    ctx.restore();
    return 1;
  },
  image_data: (p, n, d, len) => {
    const src = str(p, n);
    if (images.get(src) === undefined) loadImage(src, new Blob([new Uint8Array(wasm.memory.buffer, d, len).slice()]));
  },
  image_size: (p, n, out) => {
    const src = str(p, n);
    const im = images.get(src);
    if (im === undefined && !src.startsWith("mem:")) loadImage(src);
    if (!im) return 0;
    new Float32Array(wasm.memory.buffer, out, 2).set([im.bmp.width, im.bmp.height]);
    return 1;
  },
  measure: (p, n, f, out) => {
    setFont(f);
    const m = ctx.measureText(str(p, n));
    const a = new Float32Array(wasm.memory.buffer, out, 3);
    a[0] = m.width;
    a[1] = m.fontBoundingBoxAscent ?? m.actualBoundingBoxAscent;
    a[2] = m.fontBoundingBoxDescent ?? m.actualBoundingBoxDescent;
  },
};

// Canvas size in CSS points and pixel density. Resizing clears the canvas.
function fit() {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.lineJoin = "round";
  ctx.textBaseline = "alphabetic";
  curFont = -1;
  return [w, h, dpr];
}

const pos = (e) => {
  const r = canvas.getBoundingClientRect();
  return [e.clientX - r.left, e.clientY - r.top];
};

const KEYS = {
  Backspace: 1, Delete: 2, Enter: 3, Tab: 4, Escape: 5, ArrowLeft: 6, ArrowRight: 7,
  ArrowUp: 8, ArrowDown: 9, Home: 10, End: 11, PageUp: 12, PageDown: 13, ContextMenu: 14, F10: 15,
};

// Modifiers of the last input event (Shift+click): bits shift 1, ctrl 2, alt 4, cmd 8.
// "Primary modifier": ⌘ on macOS, Ctrl elsewhere — for ui.shortcut("cmd+n").
let curMods = 0;
const modsOf = (e) =>
  (e.shiftKey ? 1 : 0) | (isMac && e.ctrlKey ? 2 : 0) | (e.altKey ? 4 : 0) | ((isMac ? e.metaKey : e.ctrlKey) ? 8 : 0);

let pasteWanted = false;
function paste(text) {
  const bytes = enc.encode(text);
  new Uint8Array(wasm.memory.buffer, wasm.uison_input(bytes.length), bytes.length).set(bytes);
  wasm.uison_paste(bytes.length);
}

// Button for uison_pointer: finger — 9 (MouseButton::Touch: finger scrolling, no hover).
const button = (e) => (e.pointerType === "touch" ? 9 : e.button);

function listen() {
  canvas.addEventListener("pointerdown", (e) => {
    canvas.focus();
    try {
      canvas.setPointerCapture(e.pointerId); // dragging (Slider) outside the canvas
    } catch {}
    curMods = modsOf(e);
    wasm.uison_pointer(0, ...pos(e), button(e));
    e.preventDefault();
  });
  canvas.addEventListener("pointerup", (e) => wasm.uison_pointer(1, ...pos(e), button(e)));
  canvas.addEventListener("pointercancel", (e) => wasm.uison_pointer(1, -1, -1, button(e)));
  canvas.addEventListener("pointermove", (e) => wasm.uison_pointer(2, ...pos(e), 0));
  canvas.addEventListener("pointerleave", () => wasm.uison_pointer(3, 0, 0, 0));
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const k = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? canvas.clientHeight : 1;
      // DOM: deltaY > 0 — scroll down; uison: dy > 0 — content moves down.
      wasm.uison_wheel(...pos(e), -e.deltaX * k, -e.deltaY * k);
    },
    { passive: false },
  );
  window.addEventListener("keydown", (e) => {
    const code = KEYS[e.key] || 0;
    const mods = (curMods = modsOf(e));
    const text = code === 0 && [...e.key].length === 1 ? e.key : "";
    const bytes = enc.encode(text).slice(0, 256);
    new Uint8Array(wasm.memory.buffer, wasm.uison_input_buffer(), 256).set(bytes);
    const handled = wasm.uison_key(code, mods, bytes.length);
    // Cancel the browser's default action only if the app handled the key (and always for Tab).
    if (handled || code === 4) e.preventDefault();
  });
  window.addEventListener("keyup", (e) => (curMods = modsOf(e)));
  document.addEventListener("paste", (e) => {
    const t = e.clipboardData?.getData("text/plain");
    if (t) {
      pasteWanted = false;
      e.preventDefault();
      paste(t);
    }
  });
  new ResizeObserver(() => wasm.uison_resize(...fit())).observe(canvas);
  // System theme (for the `system` theme).
  const darkQuery = matchMedia("(prefers-color-scheme: dark)");
  wasm.uison_appearance(darkQuery.matches ? 1 : 0);
  darkQuery.addEventListener("change", (e) => wasm.uison_appearance(e.matches ? 1 : 0));
}

// For automated checks: logs and screenshots go to tools/serve.py.
if (params.has("test") && params.get("test") !== "0") {
  for (const level of ["log", "error"]) {
    const orig = console[level];
    console[level] = (...a) => {
      orig(...a);
      fetch("/log", { method: "POST", body: `[${level}] ${a.join(" ")}` });
    };
  }
  window.addEventListener("error", (e) => console.error("js:", e.message));
}

window.uison = {
  canvas,
  bench: (n, mode) => wasm.uison_bench(n, mode),
  draws: () => wasm.uison_draws(),
  hitmap: () => JSON.parse(str(wasm.uison_hitmap(), wasm.uison_hitmap_len())),
  capture: (name) =>
    new Promise((ok) =>
      canvas.toBlob((b) => fetch(`/capture?name=${encodeURIComponent(name)}`, { method: "POST", body: b }).then(ok)),
    ),
};

function start(t0) {
  const on = (k) => params.has(k) && params.get(k) !== "0";
  const flags = (on("stats") ? 1 : 0) | (on("flash") ? 2 : 0);
  wasm.uison_init(...fit(), flags);
  listen();
  requestAnimationFrame(() => console.log(`uison: first frame in ${Math.round(performance.now() - t0)} ms`));
  if (params.has("script")) import("./drive.js");
}

if (typeof wasmExports !== "undefined") {
  // Emscripten: the module is already created, main is running. uison_* functions come from the function table
  // by indices from Rust (uison_js_boot), memory is the glue's HEAPU8 (replaced when memory grows).
  wasm = {
    get memory() {
      return HEAPU8;
    },
  };
  // A glue variable: Emscripten shortens export names at -Os/-Oz.
  const table = __indirect_function_table;
  const ix = new Uint32Array(HEAPU8.buffer, fns, n);
  str(names, names_len)
    .split(",")
    .forEach((k, i) => k && (wasm[k] = table.get(ix[i])));
  globalThis.uisonJs = api;
  // uison_init — after leaving main (uison_run never returns, Emscripten swallows that exit).
  const t0 = performance.now();
  queueMicrotask(() => start(t0));
} else {
  const t0 = performance.now();
  const uison = Object.fromEntries(Object.entries(api).map(([k, f]) => [`uison_js_${k}`, f]));
  WebAssembly.instantiateStreaming(fetch("app.wasm"), { uison }).then(({ instance }) => {
    wasm = instance.exports;
    wasm.main(0, 0); // uison::run(app, title)
    start(t0);
  });
}
