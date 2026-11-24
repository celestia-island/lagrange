// Chrome-backed contract checks for the two docs-search properties that
// happy-dom cannot observe:
//
//   1. hydration shape — SSR emits the inline field (no viewport on the
//      server); the client must keep it on the first pass and switch to the
//      trigger AFTER mount. A client that decides the shape during setup
//      renders `sheet` against the server's `inline`, the hydration
//      mismatch keeps the server DOM, and the trigger never appears until a
//      resize event happens to fire (round 3 of #124). happy-dom's tests
//      mock `useBreakpoint`, so they cannot see this.
//   2. the fetch receiver — the component must hand DocsSearch a fetch
//      bound to the Window. In happy-dom `window === globalThis`, so
//      `bind(globalThis)` passes there while Chrome (its own realm) rejects
//      it with "Illegal invocation" (round 4 of #124).
//
// Usage: node scripts/check-search-chrome.mjs <served-base-url>
// Reuses the CDP endpoint on 127.0.0.1:9222 (never starts a browser), opens
// one tab, and closes it. Exit code 1 on any failed check.
import { writeFileSync } from "node:fs";

const base = process.argv[2];
if (!base) {
  console.error("usage: check-search-chrome.mjs <served-base-url>");
  process.exit(2);
}

const CDP = "http://127.0.0.1:9222";
const tab = await (await fetch(`${CDP}/json/new?about:blank`, { method: "PUT" })).json();
const sock = new WebSocket(tab.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
sock.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (pending.has(m.id)) {
    pending.get(m.id)(m.result ?? m.error);
    pending.delete(m.id);
  }
};
await new Promise((r) => (sock.onopen = r));
const send = (method, params = {}) =>
  new Promise((res) => {
    const i = ++id;
    pending.set(i, res);
    sock.send(JSON.stringify({ id: i, method, params }));
  });

const failures = [];
const check = (name, ok, detail) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
  if (!ok) failures.push(name);
};

async function load(path, width, height, mobile) {
  await send("Network.setCacheDisabled", { cacheDisabled: true });
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 2, mobile });
  await send("Page.navigate", { url: `${base}${path}` });
  await new Promise((r) => setTimeout(r, 2500));
}

try {
  // ── 1. phone, fresh load, NO resize event ───────────────────────────────
  await load("/en/intro.html", 412, 915, true);
  const shape = await send("Runtime.evaluate", {
    expression: `document.querySelector('.lagrange-docs-search')?.dataset.shape`,
    returnByValue: true,
  });
  check("phone hydration switches to the trigger after mount", shape.result.value === "sheet", `shape=${shape.result.value}`);

  const trigger = await send("Runtime.evaluate", {
    expression: `(() => { const b = document.querySelector("button[aria-haspopup='dialog']"); return b ? 'present' : 'absent'; })()`,
    returnByValue: true,
  });
  check("phone renders the search trigger", trigger.result.value === "present");

  const layout = await send("Runtime.evaluate", {
    expression: `(() => {
      const btns = [...document.querySelectorAll('.s-admin-header-actions button')].map((b) => Math.round(b.getBoundingClientRect().right));
      return JSON.stringify({ overflow: document.documentElement.scrollWidth > innerWidth, maxRight: Math.max(...btns) });
    })()`,
    returnByValue: true,
  });
  const l = JSON.parse(layout.result.value);
  check("phone header does not overflow", !l.overflow && l.maxRight <= 412, `maxRight=${l.maxRight}`);

  // ── 2. the receiver: a real query must answer, not error ────────────────
  await send("Runtime.evaluate", {
    expression: `document.querySelector("button[aria-haspopup='dialog']").click()`,
  });
  await new Promise((r) => setTimeout(r, 800));
  await send("Runtime.evaluate", {
    expression: `(() => { const i = document.querySelector('.lagrange-docs-search-sheet input'); i.value = 'philosophy'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  });
  await new Promise((r) => setTimeout(r, 1800));
  const search = await send("Runtime.evaluate", {
    expression: `(() => JSON.stringify({
      hits: document.querySelectorAll('.lagrange-docs-search-hit').length,
      error: !!document.querySelector('.lagrange-docs-search-error'),
      input: (() => { const b = document.querySelector('.lagrange-docs-search-sheet input').getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom)]; })(),
      clear: (() => { const e = document.querySelector('.lagrange-docs-search-sheet .lagrange-docs-search-clear'); if (!e) return null; const b = e.getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom)]; })(),
    }))()`,
    returnByValue: true,
  });
  const s = JSON.parse(search.result.value);
  check("phone search answers with hits (fetch receiver is the Window)", s.hits > 0 && !s.error, `hits=${s.hits} error=${s.error}`);
  check(
    "phone clear button sits in the input row",
    s.clear !== null && s.clear[0] >= s.input[0] && s.clear[1] <= s.input[1],
    `input=${s.input} clear=${s.clear}`,
  );

  // ── 3. desktop: results must be visible, not clipped by the header ─────
  await load("/en/intro.html", 1280, 800, false);
  const dshape = await send("Runtime.evaluate", {
    expression: `document.querySelector('.lagrange-docs-search')?.dataset.shape`,
    returnByValue: true,
  });
  check("desktop keeps the inline field", dshape.result.value === "inline", `shape=${dshape.result.value}`);
  await send("Runtime.evaluate", {
    expression: `(() => { const i = document.querySelector('.lagrange-docs-search-input'); i.value = 'philosophy'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  });
  await new Promise((r) => setTimeout(r, 1800));
  const vis = await send("Runtime.evaluate", {
    expression: `(() => {
      const hits = document.querySelectorAll('.lagrange-docs-search-hit');
      if (!hits.length) return JSON.stringify({ hits: 0, visible: false });
      // A hit is VISIBLE when the topmost element at its centre is the hit
      // itself (or inside it) — the header's overflow:hidden would put the
      // header in the way instead.
      const b = hits[0].getBoundingClientRect();
      const top = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
      return JSON.stringify({ hits: hits.length, visible: !!top && hits[0].contains(top), at: top ? top.className.toString().slice(0, 40) : null });
    })()`,
    returnByValue: true,
  });
  const v = JSON.parse(vis.result.value);
  check("desktop results are on screen, not clipped by the header", v.hits > 0 && v.visible, `hits=${v.hits} at=${v.at}`);
} finally {
  try {
    await fetch(`${CDP}/json/close/${tab.id}`);
  } catch {
    /* tab already gone */
  }
}

if (failures.length) {
  console.error(`\n${failures.length} check(s) failed: ${failures.join("; ")}`);
  process.exit(1);
}
writeFileSync("/tmp/check-search-chrome.ok", new Date().toISOString());
console.log("\nall checks passed");
