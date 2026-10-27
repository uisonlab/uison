// drive.js — a check scenario right inside the page (web counterpart of tools/drive.swift).
// A scenario is a JSON array of steps in ?script=… (or a file name next to it: ?script=studio.json):
//   ["tap", id] ["hover", id, ms] ["wheel", id, dy] ["type", "text"] ["key", "Enter", "shift+alt"]
//   ["cmd", "n"] — with the primary modifier ⌘/Ctrl, ["wait", ms], ["capture", "name"], ["done"],
//   ["click", id, n] ["rclick", id] ["hold", id] (finger) ["select", id, x0, x1] ["paste", "text"]
//   — as in uison_frame::script. Clipboard writes are printed to the log: `clipboard: text`.
// Events are real DOM events on the canvas; elements are found by id in the hit-area map.

const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const canvas = window.uison.canvas;
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));

// The last area with this id is the topmost (popups are drawn later).
function area(id) {
  const a = window.uison.hitmap().findLast((h) => h[0] === id);
  if (!a) throw new Error(`no hit area '${id}'`);
  const r = canvas.getBoundingClientRect();
  return { x: r.left + a[1], y: r.top + a[2], w: a[3], h: a[4] };
}

function center(id) {
  const a = area(id);
  return { clientX: a.x + a.w / 2, clientY: a.y + a.h / 2 };
}

const pointer = (type, p, button = 0, extra = {}) =>
  canvas.dispatchEvent(new PointerEvent(type, { ...p, button, bubbles: true, pointerId: 1, isPrimary: true, ...extra }));

// Scenario key names → DOM.
const DOM_KEYS = { Left: "ArrowLeft", Right: "ArrowRight", Up: "ArrowUp", Down: "ArrowDown" };
const modsOf = (spec = "") => {
  const m = {};
  for (const p of spec.split("+").filter(Boolean)) {
    if (p === "cmd") m[isMac ? "metaKey" : "ctrlKey"] = true;
    else m[{ shift: "shiftKey", alt: "altKey", ctrl: "ctrlKey" }[p]] = true;
  }
  return m;
};
const key = (k, extra = {}) =>
  window.dispatchEvent(new KeyboardEvent("keydown", { key: DOM_KEYS[k] || k, bubbles: true, cancelable: true, ...extra }));

// Clipboard writes go to the log (headless Chrome may refuse the real write: then the log has an error too).
if (navigator.clipboard) {
  const write = navigator.clipboard.writeText.bind(navigator.clipboard);
  navigator.clipboard.writeText = (t) => (console.log(`clipboard: ${t}`), write(t));
}

async function run(steps) {
  for (const step of steps) {
    const [op, a, b] = step;
    switch (op) {
      case "tap": {
        const p = center(a);
        pointer("pointermove", p);
        pointer("pointerdown", p);
        pointer("pointerup", p);
        break;
      }
      case "hover":
        pointer("pointermove", center(a));
        if (b) await sleep(b);
        break;
      case "click": {
        const p = center(a);
        pointer("pointermove", p);
        for (let k = 0; k < (b || 1); k++) {
          pointer("pointerdown", p);
          pointer("pointerup", p);
          await sleep(40);
        }
        break;
      }
      case "rclick": {
        const p = center(a);
        pointer("pointermove", p);
        pointer("pointerdown", p, 2);
        pointer("pointerup", p, 2);
        break;
      }
      case "hold": {
        const p = center(a);
        pointer("pointerdown", p, 0, { pointerType: "touch" });
        await sleep(600);
        pointer("pointerup", p, 0, { pointerType: "touch" });
        break;
      }
      case "select": {
        // ["select", id, x0, x1]: drag the mouse inside the element (x — from its left edge).
        const r = area(a);
        const x1 = step[3];
        const at = (x) => ({ clientX: r.x + x, clientY: r.y + r.h / 2 });
        pointer("pointermove", at(b));
        pointer("pointerdown", at(b));
        for (let k = 1; k <= 6; k++) {
          pointer("pointermove", at(b + ((x1 - b) * k) / 6));
          await sleep(16);
        }
        pointer("pointerup", at(x1));
        break;
      }
      case "paste": {
        // A real paste event — as from ⌘V / Ctrl+V.
        const data = new DataTransfer();
        data.setData("text/plain", a);
        document.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
        break;
      }
      case "wheel": {
        // As in uison_frame::script: dy < 0 — scroll down (in DOM down is deltaY > 0).
        const p = center(a);
        canvas.dispatchEvent(new WheelEvent("wheel", { ...p, deltaY: -b, deltaMode: 0, bubbles: true, cancelable: true }));
        break;
      }
      case "type":
        for (const ch of a) {
          key(ch);
          await sleep(15);
        }
        break;
      case "key":
        key(a, modsOf(b));
        break;
      case "cmd":
        key(a, isMac ? { metaKey: true } : { ctrlKey: true });
        break;
      case "wait":
        await sleep(a);
        break;
      case "capture":
        await sleep(60); // wait for a frame (requestAnimationFrame)
        await window.uison.capture(a);
        if (b) await sleep(b);
        break;
      case "bench": {
        // ["bench", n]: full frame redraw; share of text and string decoding.
        const n = a || 40;
        const res = {};
        const add = (k, v) => (res[k] = res[k] || []).push(v);
        for (let round = 0; round < 4; round++) {
          add("record", window.uison.bench(n, 0));
          add("record, no text", window.uison.bench(n, 2));
          window.uison.skipFill = true;
          add("record, text decode only", window.uison.bench(n, 0));
          window.uison.skipFill = false;
        }
        for (const [k, v] of Object.entries(res)) {
          const s = v.slice(1).sort((x, y) => x - y); // first round is warm-up
          console.log(`bench ${k}: ${s[Math.floor(s.length / 2)].toFixed(0)} us/frame`);
        }
        break;
      }
      case "moves": {
        // ["moves", [idA, idB], n]: n mouse moves alternating over two elements,
        // one per browser frame (like a real mouse) — experiment 2.
        const [pa, pb] = a.map(center);
        const d0 = window.uison.draws();
        const t = performance.now();
        for (let k = 0; k < b; k++) {
          pointer("pointermove", pa); // two changes per frame: the mouse is faster than the screen
          pointer("pointermove", pb);
          await new Promise(requestAnimationFrame);
        }
        const ms = performance.now() - t;
        await sleep(100);
        console.log(`moves: ${2 * b} events in ${ms.toFixed(0)} ms, draws ${window.uison.draws() - d0}`);
        break;
      }
      case "fps": {
        // ["fps", id, frames, dy]: scrolling over an element, one event per frame;
        // intervals between browser frames include our drawing and rasterization.
        const p = center(a);
        const n = b || 120;
        const dy = steps.find((s) => s === undefined) ?? 40;
        const gaps = [];
        let last = performance.now();
        for (let k = 0; k < n; k++) {
          const dir = k < n / 2 ? 1 : -1;
          canvas.dispatchEvent(new WheelEvent("wheel", { ...p, deltaY: dir * 40, deltaMode: 0, bubbles: true, cancelable: true }));
          await new Promise(requestAnimationFrame);
          const now = performance.now();
          gaps.push(now - last);
          last = now;
        }
        gaps.sort((x, y) => x - y);
        const avg = gaps.reduce((x, y) => x + y, 0) / gaps.length;
        console.log(`fps: ${n} frames, ${(1000 / avg).toFixed(0)} fps avg, frame p50 ${gaps[n >> 1].toFixed(1)} ms, p95 ${gaps[Math.floor(n * 0.95)].toFixed(1)} ms, max ${gaps[n - 1].toFixed(1)} ms`);
        break;
      }
      case "metrics": {
        // ["metrics", [[text, size, bold], ...]]: widths with the same font as WebText.
        const ctx = canvas.getContext("2d");
        const sys = '-apple-system, BlinkMacSystemFont, system-ui, "Segoe UI", Roboto, sans-serif';
        for (const [t, size, bold] of a) {
          ctx.font = `${bold ? 700 : 400} ${size}px ${sys}`;
          const m = ctx.measureText(t);
          console.log(`metric\t${t}\t${m.width.toFixed(2)}\t${m.fontBoundingBoxAscent.toFixed(2)}\t${m.fontBoundingBoxDescent.toFixed(2)}`);
        }
        break;
      }
      case "done":
        await fetch("/done");
        break;
    }
    await sleep(40);
  }
}

const src = new URLSearchParams(location.search).get("script");
// Shared scenarios start with "// …" comments — JSON.parse doesn't understand them.
const uncomment = (t) => t.replace(/^\s*\/\/.*$/gm, "");
const steps = JSON.parse(uncomment(src.trim().startsWith("[") ? src : await (await fetch(src)).text()));
run(steps).catch((e) => {
  console.error("drive:", e.message);
  fetch("/done");
});
