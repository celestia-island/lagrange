// Chrome-backed contract checks for the two docs-search properties that
// happy-dom cannot observe:
//
//   1. hydration shape — SSR emits an EMPTY shell (no viewport on the
//      server, so it picks no shape); the client paints the trigger or the
//      field only after mount. A client that decided the shape during setup
//      rendered the sheet against the server's field, the hydration
//      mismatch kept the server DOM, and the trigger never appeared until a
//      resize event happened to fire (rounds 3 and 5 of #124). happy-dom's
//      tests mock `useBreakpoint`, so they cannot see this.
//   2. the fetch receiver — the component must hand DocsSearch a fetch
//      bound to the Window. In happy-dom `window === globalThis`, so
//      `bind(globalThis)` passes there while Chrome (its own realm) rejects
//      it with "Illegal invocation" (round 4 of #124).
//
// Usage: node scripts/check-search-chrome.mjs <served-base-url> [page-path] [query-word]
//   page-path   — a page that exists in the build (default /en/intro.html)
//   query-word  — a word that occurs in that build's index (default "philosophy")
// Reuses the CDP endpoint on 127.0.0.1:9222 (never starts a browser), opens
// one tab, and closes it. Exit code 1 on any failed check.
import { writeFileSync } from "node:fs";

const base = process.argv[2];
const PAGE = process.argv[3] ?? "/en/intro.html";
const WORD = process.argv[4] ?? "philosophy";
if (!base) {
  console.error("usage: check-search-chrome.mjs <served-base-url> [page-path] [query-word]");
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

/** Evaluate an expression; a thrown error (e.g. a null deref on a red
 * build) becomes `null` so the check that reads it reports FAIL instead
 * of killing the run before the remaining checks and the tab cleanup. */
async function safe(expression) {
  const r = await send("Runtime.evaluate", { expression: `(() => { try { return ${expression}; } catch { return null; } })()`, returnByValue: true });
  return r?.result?.value ?? null;
}

/** Collect console errors + hydration-mismatch messages while a page runs. */
const consoleErrors = [];
sock.addEventListener("message", (ev) => {
  const m = JSON.parse(ev.data);
  if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") {
    consoleErrors.push(m.params.args.map((a) => a.value ?? a.description ?? "").join(" "));
  }
});

async function load(path, width, height, mobile) {
  consoleErrors.length = 0;
  await send("Network.setCacheDisabled", { cacheDisabled: true });
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 2, mobile });
  await send("Page.navigate", { url: `${base}${path}` });
  await new Promise((r) => setTimeout(r, 2500));
}

try {
  await send("Runtime.enable");
  // ── 1. phone, fresh load, NO resize event ───────────────────────────────
  await load(PAGE, 412, 915, true);
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

  const layout = await safe(`(() => {
    const btns = [...document.querySelectorAll('.s-admin-header-actions button')].map((b) => Math.round(b.getBoundingClientRect().right));
    return JSON.stringify({ overflow: document.documentElement.scrollWidth > innerWidth, maxRight: btns.length ? Math.max(...btns) : -1, vw: innerWidth });
  })()`);
  const l = layout ? JSON.parse(layout) : { overflow: true, maxRight: -1, vw: 0 };
  check("phone header does not overflow", !l.overflow && l.maxRight <= l.vw, `maxRight=${l.maxRight} vw=${l.vw}`);
  // The shape flip after mount must not cost a hydration mismatch: Vue's
  // own report is the oracle (round 5 of #124 found the server subtree
  // being replaced on every phone load while every other check passed).
  // Known, pre-existing, NOT this component's — and bisected to its real
  // source (round 10 of #124 corrected an earlier wrong attribution):
  // `HkFaceShell` passes `compact: !isDesktop.value` to the header, and
  // hikari's `useBreakpoint` answers `isDesktop: true` on the server (no
  // viewport). So the server emits no hamburger and a phone's first client
  // pass renders one — a structural mismatch Vue reports as one line. It
  // persists with the search component removed from the shell entirely,
  // and the language/theme buttons' `hk-icon-button-32` class is IDENTICAL
  // in the SSR string and the live DOM (so `useChromeActionSize` is NOT the
  // cause). The search's own shell is empty on both first passes (gated on
  // mount), so it adds no node of its own. Tracked for hikari.
  // One row, one size. The header's action size is viewport-derived and the
  // server has none, so a live value on the first client pass mismatches and
  // Vue keeps the server's class — the language/theme buttons stayed 32px
  // while the post-mount search trigger showed the true 28 (round 15 of
  // #124). All three must agree at rest.
  const sizes = await safe(`JSON.stringify([...document.querySelectorAll('.s-admin-header-actions button')].map((b) => String(b.className).match(/hk-icon-button-(\\d+)/)?.[1] ?? '?'))`);
  const sizeList = sizes ? JSON.parse(sizes) : [];
  check(
    "phone: every header action renders the same size",
    sizeList.length >= 2 && new Set(sizeList).size === 1,
    `sizes=${sizeList.join(",")}`,
  );

  const mismatch = consoleErrors.filter((e) => /hydrat/i.test(e));
  console.log(`info  phone hydration: ${mismatch.length} mismatch line(s) — HkFaceShell's compact/hamburger, pre-existing in hikari`);
  // The mismatch LINE cannot be this component's oracle (see above). Its
  // guard lives in vitest: `docsSearchBox.ssr.test.ts` renders the shell
  // with NO mocks in a node environment (the real server branches), so a
  // leaked child fails there — verified by mutation (rounds 10).
  // ── 2. the receiver: a real query must answer, not error ────────────────
  await send("Runtime.evaluate", {
    expression: `document.querySelector("button[aria-haspopup='dialog']").click()`,
  });
  await new Promise((r) => setTimeout(r, 800));
  // HkPopover focuses its own panel when it opens, so a real click on the
  // sheet's field is what gives it focus; then real typed text.
  const sfield = await safe(`(() => { const b = document.querySelector('.lagrange-docs-search-sheet input').getBoundingClientRect(); return JSON.stringify({ x: b.left + b.width / 2, y: b.top + b.height / 2 }); })()`);
  const sf = sfield ? JSON.parse(sfield) : { x: 0, y: 0 };
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: sf.x, y: sf.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: sf.x, y: sf.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 150));
  const sfocused = await safe(`document.activeElement === document.querySelector('.lagrange-docs-search-sheet input')`);
  check("phone: the sheet's field takes focus from a real tap", sfocused === true, `focused=${sfocused}`);
  await send("Input.insertText", { text: WORD });
  await new Promise((r) => setTimeout(r, 1800));
  const search = await safe(`JSON.stringify({
    hits: document.querySelectorAll('.lagrange-docs-search-hit').length,
    error: !!document.querySelector('.lagrange-docs-search-error'),
    input: (() => { const b = document.querySelector('.lagrange-docs-search-sheet input').getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom)]; })(),
    clear: (() => { const e = document.querySelector('.lagrange-docs-search-sheet .lagrange-docs-search-clear'); if (!e) return null; const b = e.getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom)]; })(),
  })`);
  const s = search ? JSON.parse(search) : { hits: 0, error: true, input: null, clear: null };
  check("phone search answers with hits (fetch receiver is the Window)", s.hits > 0 && !s.error, `hits=${s.hits} error=${s.error}`);
  check(
    "phone clear button sits in the input row",
    s.clear !== null && s.clear[0] >= s.input[0] && s.clear[1] <= s.input[1],
    `input=${s.input} clear=${s.clear}`,
  );

  // ── 3. desktop: results must be visible, not clipped by the header ─────
  await load(PAGE, 1280, 800, false);
  const dshape = await safe(`document.querySelector('.lagrange-docs-search')?.dataset.shape`);
  check("desktop keeps the inline field", dshape === "inline", `shape=${dshape}`);
  // "no mismatch" is only meaningful if the client actually hydrated:
  // consoleErrors is empty both when hydration is clean AND when no bundle
  // ran at all (round 12 measured the check printing ok with the client JS
  // deleted). Assert the app mounted first.
  const dhydrated = await safe(`!!document.querySelector('.lagrange-docs-search')?.dataset.shape`);
  check("desktop: the client bundle actually hydrated", dhydrated === true, `hydrated=${dhydrated}`);
  check("desktop hydration reports no mismatch", !consoleErrors.some((e) => /hydrat/i.test(e)), consoleErrors.find((e) => /hydrat/i.test(e)) ?? "");

  // A REAL pointer focus on the field, then real typed text. The focus
  // matters: the hit-click regression is a blur-then-close sequence, and a
  // panel opened by a programmatic value change (no focused input) can
  // never blur — a mutant with the old bug passed this very check until
  // the focus was made real (round 8 of #124).
  const field = await safe(`(() => { const b = document.querySelector('.lagrange-docs-search-input').getBoundingClientRect(); return JSON.stringify({ x: b.left + b.width / 2, y: b.top + b.height / 2 }); })()`);
  const f = field ? JSON.parse(field) : { x: 0, y: 0 };
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: f.x, y: f.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: f.x, y: f.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 150));
  const focused = await safe(`document.activeElement === document.querySelector('.lagrange-docs-search-input')`);
  check("desktop: the field takes focus from a real click", focused === true, `focused=${focused}`);
  await send("Input.insertText", { text: WORD });
  await new Promise((r) => setTimeout(r, 1800));

  // The clear button must sit INSIDE the field's row on desktop too: it is
  // absolutely positioned against `.lagrange-docs-search-field`, and moving
  // it out of that wrapper leaves it floating over the results (round 10 of
  // #124 measured it outside the field at 1280×800 with nothing noticing).
  const dclear = await safe(`(() => {
    const i = document.querySelector('.lagrange-docs-search-input').getBoundingClientRect();
    const c = document.querySelector('.lagrange-docs-search-clear');
    if (!c) return JSON.stringify({ present: false });
    const b = c.getBoundingClientRect();
    return JSON.stringify({ present: true, inside: b.top >= i.top && b.bottom <= i.bottom && b.right <= i.right + 1, input: [Math.round(i.top), Math.round(i.bottom)], clear: [Math.round(b.top), Math.round(b.bottom)] });
  })()`);
  const dc = dclear ? JSON.parse(dclear) : { present: false };
  check("desktop clear button sits in the input row", dc.present && dc.inside, `input=${dc.input} clear=${dc.clear}`);

  const vis = await safe(`(() => {
    const hits = document.querySelectorAll('.lagrange-docs-search-hit');
    if (!hits.length) return JSON.stringify({ hits: 0, visible: false, at: null, focused: document.activeElement?.className });
    const b = hits[0].getBoundingClientRect();
    // A hit is ON SCREEN when the topmost element at its centre is the hit
    // itself (or inside it) — the header's overflow:hidden would put the
    // header in the way instead.
    const top = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
    return JSON.stringify({ hits: hits.length, visible: !!top && hits[0].contains(top), at: top ? String(top.className).slice(0, 40) : null, cx: b.left + b.width / 2, cy: b.top + b.height / 2, href: hits[0].getAttribute('href'), focused: document.activeElement?.className });
  })()`);
  const v = vis ? JSON.parse(vis) : { hits: 0, visible: false, at: null, cx: 0, cy: 0, href: null, focused: null };
  check("desktop results are on screen, not clipped by the header", v.hits > 0 && v.visible, `hits=${v.hits} at=${v.at}`);
  // The field MUST still hold focus when the hits are on screen — that is
  // the precondition for the blur the click below will cause.
  check("desktop: the field keeps focus while results show", typeof v.focused === "string" && v.focused.includes("lagrange-docs-search-input"), `focused=${v.focused}`);

  // ── 4. desktop: a REAL mouse click on a hit navigates ──────────────────
  // The regression this pins (round 5 of #124): the results are teleported
  // outside the field's box, and a containment test in a `focusout` handler
  // called every hit "outside" — mousedown blurred the input, the panel
  // closed, and the anchor was gone before `click`. Only a real pointer
  // sequence FROM A FOCUSED FIELD (mousedown → blur → mouseup) reproduces
  // it; a synthetic `.click()` does not, and neither does an unfocused field.
  const before = await safe(`location.pathname + location.hash`);
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: v.cx, y: v.cy });
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: v.cx, y: v.cy, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 60));
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: v.cx, y: v.cy, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 1500));
  const after = await safe(`location.pathname + location.hash`);
  // Navigate = the location changed AND it is the hit's own target (the
  // href may carry a hash; the path must match).
  const hrefPath = (v.href ?? "").split("#")[0];
  const navigated = !!v.href && after !== before && after.startsWith(hrefPath);
  check("desktop: clicking a hit navigates to it", navigated, `before=${before} after=${after} href=${v.href}`);
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
