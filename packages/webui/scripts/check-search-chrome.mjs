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
//   page-path   — a page that exists in the build. Default: auto-discovered
//                 from the served search_meta.json (the `/index.html`
//                 document of the index's first language, served at
//                 `/<lang><doc.url>`; falls back to that language's first
//                 document when no index.html exists) — the hard-coded
//                 "/en/intro.html" predates the deployed corpus, whose
//                 English entry is "/en/index.html".
//   query-word  — a word that occurs in that build's index. Default:
//                 auto-discovered from the served search_meta.json — the
//                 word with the most postings on the page's language, so
//                 the default works on ANY corpus (the hard-coded
//                 "philosophy" predated the tiny 3-page lagrange corpus
//                 and turned every default run into 3 false-reds).
//                 Discovery is the index's own lookup, inverted: postings
//                 are [docId, chunk] pairs and the same lang filter
//                 `search.ts` applies is applied here, so a discovered
//                 word cannot answer zero hits.
// Reuses the CDP endpoint on 127.0.0.1:9222 (never starts a browser), opens
// one tab, and closes it. On a full pass it writes /tmp/check-search-chrome.ok
// (cleared at start, so it never survives a failed or crashed run).
// Exit 1 = at least one check failed; exit 2 =
// the run could not start (no base URL, undiscoverable page/word, shard
// count mismatch, or no reachable CDP endpoint).
import { unlinkSync, writeFileSync } from "node:fs";

const base = process.argv[2];

/** Everything the served build publishes: docs keyed by language, the
 * shard names, and each shard's postings. Shared by both discoveries —
 * one fetch of the meta, shards pulled lazily. */
async function loadSearchIndex(base) {
  try {
    const res = await fetch(`${base}/search_meta.json`, { signal: AbortSignal.timeout(10000) });
    if (!res.ok) return null;
    const meta = await res.json();
    if (!Array.isArray(meta?.docs) || !Array.isArray(meta?.shards)) return null;
    const byLang = new Map();
    for (const doc of meta.docs) {
      if (!byLang.has(doc.lang)) byLang.set(doc.lang, []);
      byLang.get(doc.lang).push(doc);
    }
    return { meta, byLang };
  } catch {
    return null;
  }
}

/** The page the checks should load: the index's own entry document for a
 * language the build actually ships (the language root `/<lang>/` serves
 * it). Doc urls are flat (`/index.html`) per the build contract. */
async function discoverPage(base) {
  const index = await loadSearchIndex(base);
  const docs = index?.byLang?.get([...(index?.byLang?.keys() ?? [])][0]);
  const doc = docs?.find((d) => d.url === "/index.html") ?? docs?.[0];
  return doc ? `/${doc.lang}${doc.url}` : null;
}

/** Pick a query word the served build is guaranteed to answer: the
 * lowercase alphanumeric token (the exact shape `tokenizeQuery` passes
 * through unchanged) with the most postings on the page language. */
async function discoverWord(base, pagePath) {
  const index = await loadSearchIndex(base);
  if (!index) return null;
  // Match the page against the build's OWN language segments first — a
  // naive `[a-z]{2,8}` slice mangles multi-segment tags like `zh-Hans`
  // (it would extract "index" out of `/zh-Hans/index.html`).
  const knownLangs = [...index.byLang.keys()];
  const path = `/${pagePath.replace(/^\/+/, "")}`;
  const lang =
    knownLangs.find((k) => path.toLowerCase().startsWith(`/${k.toLowerCase()}/`)) ??
    knownLangs.find((k) => path.toLowerCase() === `/${k.toLowerCase()}`);
  if (!lang || !index.byLang.has(lang)) return null;
  const langDocs = new Set((index.byLang.get(lang) ?? []).map((d) => d.id));
  if (!langDocs.size) return null;
  let best = null;
  // The CLIENT hard-codes its shard count (`SHARD_COUNT = 16` in
  // `docsShell/search.ts`, mirroring `src/search.rs`'s constant), so
  // addressing parity holds only when the served index really has 16
  // shards. If a future build changes that count, discovery must refuse
  // rather than compute an index the client would never look in.
  const shardCount = index.meta.shards.length;
  if (shardCount !== 16) {
    console.error(`the served index has ${shardCount} shards but the client addresses 16 — cannot prove a discovered word lands in the shard the client reads`);
    process.exit(2);
  }
  for (const [shardIdx, name] of index.meta.shards.entries()) {
    let shard;
    try {
      const res = await fetch(`${base}/${name}`, { signal: AbortSignal.timeout(10000) });
      if (!res.ok) continue;
      shard = await res.json();
    } catch {
      continue;
    }
    for (const [word, postings] of Object.entries(shard)) {
      if (!/^[a-z][a-z0-9]+$/.test(word) || !Array.isArray(postings)) continue;
      // The client looks a word up in `shardFor(word)` = charCodeAt(0) %
      // shardCount, NOT in the shard it happened to be served from. Require
      // the two to agree, so the "cannot answer zero hits" guarantee is
      // proved against the client's own addressing rather than assuming the
      // build shards words the same way the client reads them.
      const clientShard = (word.codePointAt(0) ?? 63) % shardCount;
      if (clientShard !== shardIdx) continue;
      const inLang = postings.filter(([docId]) => langDocs.has(docId)).length;
      if (inLang > 0 && (!best || inLang > best[1])) best = [word, inLang];
    }
  }
  return best?.[0] ?? null;
}

// The success sentinel must not survive from an EARLIER run: a crashed or
// failed run would leave a passing marker in place and read as green.
// Removed before ANY exit path (usage / discovery / CDP failures below), so
// every way of not finishing clears it; only a full pass writes it again.
const OK_SENTINEL = "/tmp/check-search-chrome.ok";
try {
  unlinkSync(OK_SENTINEL);
} catch {
  /* nothing to clear */
}

if (!base) {
  console.error("usage: check-search-chrome.mjs <served-base-url> [page-path] [query-word]");
  process.exit(2);
}
const PAGE = process.argv[3] ?? (await discoverPage(base));
const WORD = process.argv[4] ?? (PAGE ? await discoverWord(base, PAGE) : null);
if (!PAGE) {
  console.error(
    "no page path given and none could be discovered from the served search_meta.json — pass [page-path] explicitly",
  );
  process.exit(2);
}
if (!WORD) {
  console.error(
    "no query word given and none could be discovered from the served search_meta.json — pass [query-word] explicitly",
  );
  process.exit(2);
}
console.log(`info  query word: ${WORD}`);

const CDP = "http://127.0.0.1:9222";
// Acquiring the tab is the first thing that can fail, and it fails fast and
// LOUD — a raw `fetch failed` stack trace reads as a script bug rather than
// "no browser is listening", which is the overwhelmingly common cause. The
// AbortSignal bounds a connect-then-hang endpoint, which would otherwise
// stall before any socket exists.
const tab = await (async () => {
  try {
    const res = await fetch(`${CDP}/json/new?about:blank`, { method: "PUT", signal: AbortSignal.timeout(15000) });
    const created = await res.json();
    if (!created?.webSocketDebuggerUrl) {
      // The tab exists but is unusable — close it before bailing out, since
      // this exit happens before the `finally` that owns cleanup.
      await fetch(`${CDP}/json/close/${created?.id}`, { signal: AbortSignal.timeout(3000) }).catch(() => {});
      throw new Error("the CDP tab reply carried no webSocketDebuggerUrl");
    }
    return created;
  } catch (e) {
    console.error(`cannot reach the CDP endpoint ${CDP} (${e?.cause?.code ?? e?.message ?? e}) — start a browser with --remote-debugging-port=9222 first`);
    process.exit(2);
  }
})();
let sock;
let id = 0;
const pending = new Map();
/** Close the debugger tab we opened; called from the one `finally` so no
 * path (including a refused websocket) leaves it behind. */
async function closeTab() {
  try {
    if (sock) sock.close();
    await fetch(`${CDP}/json/close/${tab.id}`, { signal: AbortSignal.timeout(3000) });
  } catch {
    /* tab already gone */
  }
}
const send = (method, params = {}) =>
  new Promise((res, reject) => {
    if (!sock || sock.readyState !== 1) {
      reject(new Error(`debugger socket is not open (readyState=${sock?.readyState}) — cannot send ${method}`));
      return;
    }
    const i = ++id;
    const timer = setTimeout(() => {
      if (pending.delete(i)) reject(new Error(`debugger reply to ${method} timed out after 15s`));
    }, 15000);
    pending.set(i, (v) => {
      clearTimeout(timer);
      res(v);
    });
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

/** Collect console errors + hydration-mismatch messages while a page runs.
 * Registered on the socket inside the main `try` (the socket is opened
 * there so `finally` can close it). */
const CONSOLE_PROBE = "[probe] console channel live";
const consoleErrors = [];
function collectConsole(ev) {
  const m = JSON.parse(ev.data);
  // Vue's hydration complaints are warnings in some builds and errors in
  // others; collecting both keeps the "no mismatch" oracle independent of
  // which level the framework picks (a level change must not silently make
  // the oracle vacuous).
  if (m.method === "Runtime.consoleAPICalled" && (m.params.type === "error" || m.params.type === "warning")) {
    consoleErrors.push(m.params.args.map((a) => a.value ?? a.description ?? "").join(" "));
  }
}

/** Poll `read()` until it returns truthy, bounded — cold-sandbox first
 * loads hydrate in seconds, not the fixed-sleep 2.5s the warm preview
 * needed; fixed sleeps turned every cold run into false-reds. */
async function poll(read, tries = 20, stepMs = 500) {
  for (let i = 0; i < tries; i++) {
    const v = await read();
    if (v) return v;
    await new Promise((r) => setTimeout(r, stepMs));
  }
  return await read();
}

async function load(path, width, height, mobile) {
  consoleErrors.length = 0;
  await send("Network.setCacheDisabled", { cacheDisabled: true });
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 2, mobile });
  await send("Page.navigate", { url: `${base}${path}` });
  // Hydration oracle: the shell is EMPTY on the server and the shape is
  // the client's first mount decision — poll until the app has mounted
  // instead of guessing a fixed duration.
  await poll(async () => {
    const s = await safe(`document.querySelector('.lagrange-docs-search')?.dataset.shape ?? null`);
    return s && s !== "null" ? s : null;
  });
  // No settle delay here on purpose: the console oracle below needs Vue's
  // hydration-mismatch line, and hydration runs BEFORE the mounted hook
  // that sets `dataset.shape` — so by the time this poll succeeds the line
  // has already been emitted. Verified, not assumed: collapsing the old
  // 2.5s sleep to 0 still captures the real phone mismatch line
  // (`HkFaceShell`'s, exactly 1), so the oracle is intact and this file
  // does not re-introduce the fixed-sleep pattern that caused the
  // false-reds it replaces (round 5 of #124 made that line an oracle).
}

try {
  // The socket is opened INSIDE the try so `finally` covers a refused or
  // timed-out websocket too — otherwise the tab created above leaks.
  sock = new WebSocket(tab.webSocketDebuggerUrl);
  sock.onclose = () => {
    for (const [i, res] of pending) {
      res(undefined);
      pending.delete(i);
    }
  };
  sock.onmessage = (ev) => {
    collectConsole(ev);
    const m = JSON.parse(ev.data);
    if (pending.has(m.id)) {
      pending.get(m.id)(m.result ?? m.error);
      pending.delete(m.id);
    }
  };
  // A dead or unresponsive CDP endpoint must fail loud and fast: without a
  // bound this await never settles (the socket neither opens nor errors on
  // some failures) and the run hangs instead of reporting.
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`CDP endpoint ${CDP} did not open a websocket within 15s`)), 15000);
    sock.onopen = () => {
      clearTimeout(timer);
      resolve();
    };
    sock.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`CDP endpoint ${CDP} refused the websocket — is a browser running with --remote-debugging-port=9222?`));
    };
  });
  await send("Runtime.enable");
  // Positive control for the absence-based oracles below ("no hydration
  // mismatch"): they pass both when hydration is clean AND when the console
  // channel never delivers anything. Emit a known line and require the
  // collector to have seen it, so "0 mismatches" means measured, not blind.
  await send("Runtime.evaluate", { expression: `console.error('${CONSOLE_PROBE}')` });
  const probeSeen = await poll(async () => {
    const hit = consoleErrors.some((e) => e.includes(CONSOLE_PROBE));
    return hit ? hit : null;
  }, 6, 200);
  check("console channel delivers (control for the mismatch oracles)", probeSeen === true, `probeSeen=${probeSeen}`);
  consoleErrors.length = 0;
  // ── 1. phone, fresh load, NO resize event ───────────────────────────────
  await load(PAGE, 412, 915, true);
  const shape = await send("Runtime.evaluate", {
    expression: `document.querySelector('.lagrange-docs-search')?.dataset.shape`,
    returnByValue: true,
  });
  check("phone hydration switches to the trigger after mount", shape.result.value === "trigger", `shape=${shape.result.value}`);

  const trigger = await send("Runtime.evaluate", {
    // Present but collapsed (0×0, hidden) is the failure mode an
    // existence-only probe cannot see: require a real box.
    expression: `(() => { const b = document.querySelector(".lagrange-docs-search button[aria-haspopup='dialog']"); if (!b) return 'absent'; const r = b.getBoundingClientRect(); return r.width > 0 && r.height > 0 ? 'visible' : 'collapsed ' + Math.round(r.width) + 'x' + Math.round(r.height); })()`,
    returnByValue: true,
  });
  check("phone renders the search trigger", trigger.result.value === "visible", `trigger=${trigger.result.value}`);

  const layout = await safe(`(() => {
    const btns = [...document.querySelectorAll('.s-admin-header-actions button')].map((b) => Math.round(b.getBoundingClientRect().right));
    return JSON.stringify({ count: btns.length, overflow: document.documentElement.scrollWidth > innerWidth, maxRight: btns.length ? Math.max(...btns) : -1, vw: innerWidth });
  })()`);
  const l = layout ? JSON.parse(layout) : { count: 0, overflow: true, maxRight: -1, vw: 0 };
  // Precondition, not decoration: with no buttons rendered `maxRight` is -1
  // and "does not overflow" would pass with the header missing entirely.
  // One action is enough for THIS check (a header may legitimately carry a
  // single action); the size check below needs ≥2 because it compares.
  check("phone header does not overflow", l.count >= 1 && !l.overflow && l.maxRight <= l.vw, `buttons=${l.count} maxRight=${l.maxRight} vw=${l.vw}`);
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
  // Every action must carry a READABLE numeric size: a renamed or dropped
  // `hk-icon-button-<n>` class yields '?' for all of them, and a bare
  // `new Set(...).size === 1` would then compare one shared '?' and pass —
  // measuring nothing (round 2 of #125).
  const numeric = sizeList.length >= 2 && sizeList.every((s) => /^\d+$/.test(s));
  const phoneUniform = numeric && new Set(sizeList).size === 1;
  check(
    "phone: every header action renders the same size",
    phoneUniform,
    `sizes=${sizeList.join(",")}`,
  );

  const phoneHydrated = await safe(`!!document.querySelector('#app')?.__vue_app__`);
  check("phone: the client bundle actually hydrated", phoneHydrated === true, `vueApp=${phoneHydrated}`);
  const mismatch = consoleErrors.filter((e) => /hydrat/i.test(e));
  console.log(`info  phone hydration: ${mismatch.length} mismatch line(s) — HkFaceShell's compact/hamburger, pre-existing in hikari`);
  // The known line cannot be this component's oracle (see above); its guard
  // lives in vitest (`docsSearchBox.ssr.test.ts`, no-mock node env, mutation-
  // verified in rounds 10). But an UNBOUNDED count hides a SECOND, new
  // mismatch — the search's own — behind the tolerated one, so the oracle
  // here is "no more than the tracked defect", not "however many lines".
  // Not just a count: a SEARCH-caused line is rejected outright, so it
  // cannot substitute for HkFaceShell's tracked one and still satisfy "at
  // most one". (Other non-search lines are still tolerated — the tracked
  // defect is identified by its component, not by exact text.)
  const fromSearch = mismatch.filter((line) => /lagrange-docs-search|DocsSearch|docs-search/i.test(line));
  check(
    "phone: no hydration mismatch beyond the tracked hikari one",
    mismatch.length <= 1 && fromSearch.length === 0,
    `mismatch=${mismatch.length} fromSearch=${fromSearch.length}`,
  );
  // ── 2. the receiver: a real query must answer, not error ────────────────
  await send("Runtime.evaluate", {
    expression: `document.querySelector(".lagrange-docs-search button[aria-haspopup='dialog']").click()`,
  });
  // The modal opens synchronously on the click; poll for the search field
  // instead of assuming a fixed open latency on a cold tab. Budget 8s: a
  // tap fired before the field exists lands at (0,0) and fails everything
  // downstream (measured: a cold sandbox needed >4s here once).
  await poll(async () => {
    const p = await safe(`!!document.querySelector('.lagrange-docs-search-panel input')`);
    return p === true ? p : null;
  }, 16);
  // HkModal focuses its own surface when it opens, so a real click on the
  // search field is what gives it focus; then real typed text. The modal's
  // field is inside the dialog body — poll until it is inside the viewport
  // and settle before tapping.
  const sfield = await poll(async () => {
    const s = await safe(`(() => { const b = document.querySelector('.lagrange-docs-search-panel input').getBoundingClientRect(); return JSON.stringify({ x: b.left + b.width / 2, y: b.top + b.height / 2, inVp: b.top >= 0 && b.bottom <= innerHeight && b.width > 0 }); })()`);
    const p = s ? JSON.parse(s) : null;
    return p && p.inVp ? { x: p.x, y: p.y } : null;
  });
  const sf = sfield ?? { x: 0, y: 0 };
  // Attribution: the poll below accepts focus arriving from ANY source, so
  // prove the field did not already hold it — otherwise a poll after the tap
  // would pass on focus that preceded it.
  // Clear any focus the popover itself placed. No "was unfocused" read is
  // asserted here on purpose: this branch blurs first, so such an assertion
  // would mostly measure our own blur. Attribution comes from the sequence —
  // any focus observed from here on arrived after the tap (the poll accepts a
  // re-focus, which is the one thing this cannot rule out).
  await safe(`(() => { document.activeElement?.blur?.(); return true; })()`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: sf.x, y: sf.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: sf.x, y: sf.y, button: "left", clickCount: 1 });
  // Focus is synchronous on mousedown, so poll instead of sleeping: a fixed
  // settle here is the same false-red class this PR removes elsewhere.
  const sfocused = await poll(async () => {
    const f = await safe(`document.activeElement === document.querySelector('.lagrange-docs-search-panel input')`);
    return f === true ? f : null;
  }, 8);
  check("phone: the search field takes focus from a real tap", sfocused === true, `focused=${sfocused}`);
  await send("Input.insertText", { text: WORD });
  // The answer lands after debounce + shard fetch — poll for either
  // outcome (hits or the error state) instead of a fixed sleep.
  await poll(async () => {
    const s = await safe(`JSON.stringify({ h: document.querySelectorAll('.lagrange-docs-search-hit').length, e: !!document.querySelector('.lagrange-docs-search-error') })`);
    const p = s ? JSON.parse(s) : null;
    return p && (p.h > 0 || p.e) ? p : null;
  });
  const search = await safe(`JSON.stringify({
    hits: document.querySelectorAll('.lagrange-docs-search-hit').length,
    anyHitHasWord: [...document.querySelectorAll('.lagrange-docs-search-hit')].some((h) => (h.textContent ?? '').toLowerCase().includes(${JSON.stringify(WORD.toLowerCase())})),
    error: !!document.querySelector('.lagrange-docs-search-error'),
    input: (() => { const b = document.querySelector('.lagrange-docs-search-panel input').getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom)]; })(),
    clear: (() => { const e = document.querySelector('.lagrange-docs-search-panel .hk-search-input-clear'); if (!e) return null; const b = e.getBoundingClientRect(); return [Math.round(b.top), Math.round(b.bottom), Math.round(b.right)]; })(),
    inputRight: (() => { const b = document.querySelector('.lagrange-docs-search-panel input').getBoundingClientRect(); return Math.round(b.right); })(),
  })`);
  const s = search ? JSON.parse(search) : { hits: 0, error: true, input: null, clear: null };
  // Bind the answer to the QUESTION: a box that lists every doc regardless of
  // the typed word would otherwise satisfy a bare "hits > 0".
  check(
    "phone search answers with hits for the typed word",
    s.hits > 0 && !s.error && s.anyHitHasWord === true,
    `hits=${s.hits} error=${s.error} anyHitHasWord=${s.anyHitHasWord}`,
  );
  check(
    "phone clear button sits in the input row",
    s.clear !== null && s.clear[0] >= s.input[0] && s.clear[1] <= s.input[1] && s.clear[2] <= s.inputRight + 1,
    `input=${s.input} clear=${s.clear} inputRight=${s.inputRight}`,
  );

  // ── 3. desktop: results must be visible, not clipped by the header ─────
  await load(PAGE, 1280, 800, false);
  const dshape = await safe(`document.querySelector('.lagrange-docs-search')?.dataset.shape`);
  check("desktop wears the trigger (compact action row)", dshape === "trigger", `shape=${dshape}`);
  const dTrigRaw = await safe(`(() => { const b = document.querySelector(".lagrange-docs-search button[aria-haspopup='dialog']")?.getBoundingClientRect(); if (!b) return null; return JSON.stringify({ x: b.left + b.width / 2, y: b.top + b.height / 2 }); })()`);
  const dTrig = dTrigRaw ? JSON.parse(dTrigRaw) : { x: 0, y: 0 };
  // "no mismatch" is only meaningful if the client actually hydrated:
  // consoleErrors is empty both when hydration is clean AND when no bundle
  // ran at all (round 12 measured the check printing ok with the client JS
  // deleted). Assert the app mounted first — via a signal INDEPENDENT of
  // the shape attribute the check above already reads: `createSSRApp(...)
  // .mount("#app")` sets `__vue_app__` on the container, so this fails if
  // the bundle never ran even when a server-rendered shell looks right.
  const dhydrated = await safe(`!!document.querySelector('#app')?.__vue_app__`);
  check("desktop: the client bundle actually hydrated", dhydrated === true, `vueApp=${dhydrated}`);
  // Re-control the console channel for THIS phase: `load()` cleared the
  // collector, so the phone-phase control says nothing about desktop.
  await send("Runtime.evaluate", { expression: `console.error('${CONSOLE_PROBE}')` });
  const dProbe = await poll(async () => {
    const hit = consoleErrors.some((e) => e.includes(CONSOLE_PROBE));
    return hit ? hit : null;
  }, 6, 200);
  check("desktop: console channel delivers (control for the mismatch oracle)", dProbe === true, `probeSeen=${dProbe}`);
  // Do NOT clear the collector here: this phase's own console output (the
  // hydration-mismatch oracle below reads it) was collected by `load()`.
  // The probe text does not match `/hydrat/i`, so it needs no removal —
  // clearing here would make the oracle read an empty array and pass blind.
  check("desktop hydration reports no mismatch", !consoleErrors.some((e) => /hydrat/i.test(e)), consoleErrors.find((e) => /hydrat/i.test(e)) ?? "");

  const dSizes = await safe(`JSON.stringify([...document.querySelectorAll('.s-admin-header-actions button')].map((b) => String(b.className).match(/hk-icon-button-(\\d+)/)?.[1] ?? '?'))`);
  const dSizeList = dSizes ? JSON.parse(dSizes) : [];
  const dNumeric = dSizeList.length >= 2 && dSizeList.every((s) => /^\d+$/.test(s));
  const desktopUniform = dNumeric && new Set(dSizeList).size === 1;
  // Uniform on both sides is not enough: a phone row stuck at the DESKTOP
  // size is exactly the 28/32 regression class (uniform 32 everywhere would
  // otherwise read as "consistent"). Mobile actions must be strictly
  // smaller, which no single-viewport assertion can establish.
  check(
    "phone actions are smaller than desktop actions",
    phoneUniform && desktopUniform && Number(sizeList[0]) < Number(dSizeList[0]),
    `phone=${sizeList[0]} desktop=${dSizeList[0]}`,
  );

  // A REAL pointer focus on the field, then real typed text. The field now
  // lives INSIDE the trigger's popup (trigger-first, 2026-11-24): open the
  // popup with a real click on the trigger, wait for the field to mount in
  // the viewport, then click IT. The focus still matters: the hit-click
  // regression is a blur-then-close sequence, and a panel opened by a
  // programmatic value change (no focused input) can never blur — a mutant
  // with the old bug passed this very check until the focus was made real
  // (round 8 of #124).
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: dTrig.x, y: dTrig.y });
  await new Promise((r) => setTimeout(r, 150));
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: dTrig.x, y: dTrig.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 80));
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: dTrig.x, y: dTrig.y, button: "left", clickCount: 1 });
  const field = await poll(async () => {
    const s = await safe(`(() => { const b = document.querySelector('.lagrange-docs-search-panel .hk-search-input-field')?.getBoundingClientRect(); if (!b || b.width === 0) return null; return JSON.stringify({ x: b.left + b.width / 2, y: b.top + b.height / 2, inVp: b.top >= 0 && b.bottom <= innerHeight }); })()`);
    const p = s ? JSON.parse(s) : null;
    return p && p.inVp ? { x: p.x, y: p.y } : null;
  });
  const f = field ?? { x: 0, y: 0 };
  // Same attribution guard as the phone branch: a fresh load leaves the
  // field unfocused, and asserting that makes the click the only
  // explanation for the focus the poll below waits for.
  const fPreexisting = await safe(`(() => { const el = document.querySelector('.lagrange-docs-search-panel .hk-search-input-field'); return el ? document.activeElement === el : null; })()`);
  check("desktop: the field is unfocused before the click", fPreexisting === false, `preFocused=${fPreexisting}`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: f.x, y: f.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: f.x, y: f.y, button: "left", clickCount: 1 });
  const focused = await poll(async () => {
    const v = await safe(`document.activeElement === document.querySelector('.lagrange-docs-search-panel .hk-search-input-field')`);
    return v === true ? v : null;
  }, 8);
  check("desktop: the field takes focus from a real click", focused === true, `focused=${focused}`);
  await send("Input.insertText", { text: WORD });
  await poll(async () => {
    const s = await safe(`document.querySelectorAll('.lagrange-docs-search-hit').length`);
    return typeof s === "number" && s > 0 ? s : null;
  });

  // The clear button must sit INSIDE the field's row on desktop too: it is
  // a flex item of the standard component's row box (`.hk-search-input`),
  // so the row is THAT box — a drift that pushed the clear out of the row
  // (the round-10-of-#124 failure measured it floating over the results at
  // 1280×800 with nothing noticing) would still show up as its rect leaving
  // the box.
  const dclear = await safe(`(() => {
    const i = document.querySelector('.lagrange-docs-search-panel .hk-search-input').getBoundingClientRect();
    const c = document.querySelector('.lagrange-docs-search-panel .hk-search-input-clear');
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
    const anyHasWord = [...hits].some((h) => (h.textContent ?? '').toLowerCase().includes(${JSON.stringify(WORD.toLowerCase())}));
    return JSON.stringify({ hits: hits.length, anyHasWord, visible: !!top && hits[0].contains(top), at: top ? String(top.className).slice(0, 40) : null, cx: b.left + b.width / 2, cy: b.top + b.height / 2, href: hits[0].getAttribute('href'), focused: document.activeElement?.className });
  })()`);
  const v = vis ? JSON.parse(vis) : { hits: 0, anyHasWord: false, visible: false, at: null, cx: 0, cy: 0, href: null, focused: null };
  check(
    "desktop results are on screen, not clipped by the header",
    v.hits > 0 && v.visible && v.anyHasWord === true,
    `hits=${v.hits} anyHasWord=${v.anyHasWord} at=${v.at}`,
  );
  // NEGATIVE control, placed while results are on screen: a query that cannot
  // match must answer nothing. Binding the positive case to the word is not
  // enough — an index that ignores the query and lists every document would
  // still contain the word in SOME row (`.some()`). The discriminator is the
  // transition: the list is populated now, a nonsense token must empty it AND
  // raise the component's own "no results" state.
  const NONSENSE = "zzzqx" + "zzqx"; // one token of a shape no shard contains
  await safe(`(() => { const i = document.querySelector('.lagrange-docs-search-panel .hk-search-input-field'); if (!i) return null; const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set; set.call(i, ''); i.dispatchEvent(new Event('input', { bubbles: true })); i.focus(); return true; })()`);
  await send("Input.insertText", { text: NONSENSE });
  const negState = await poll(async () => {
    const s = await safe(`JSON.stringify({
      h: document.querySelectorAll('.lagrange-docs-search-hit').length,
      empty: !!document.querySelector('.lagrange-docs-search-empty'),
      q: document.querySelector('.lagrange-docs-search-panel .hk-search-input-field')?.value ?? '',
    })`);
    const p2 = s ? JSON.parse(s) : null;
    // Settled only when the component has ANSWERED with its empty state —
    // "h === 0" alone is true the instant the field is cleared, which would
    // let a corpus-listing search pass.
    return p2 && p2.q === NONSENSE && p2.h === 0 && p2.empty ? p2 : null;
  }, 20);
  check(
    "desktop search filters (a nonsense query answers nothing)",
    negState !== null && negState.h === 0,
    negState ? `hits=0 empty=true` : `never reached the empty state (hits/empty unresolved)`,
  );

  // Restore the positive query so the click test below runs against real hits
  // again (the negative control emptied the list).
  await safe(`(() => { const i = document.querySelector('.lagrange-docs-search-panel .hk-search-input-field'); if (!i) return null; const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set; set.call(i, ''); i.dispatchEvent(new Event('input', { bubbles: true })); i.focus(); return true; })()`);
  await send("Input.insertText", { text: WORD });
  await poll(async () => {
    const h = await safe(`document.querySelectorAll('.lagrange-docs-search-hit').length`);
    return typeof h === "number" && h > 0 ? h : null;
  });

  // The field MUST still hold focus when the hits are on screen — that is
  // the precondition for the blur the click below will cause.
  check("desktop: the field keeps focus while results show", v.hits > 0 && typeof v.focused === "string" && v.focused.includes("hk-search-input-field"), `hits=${v.hits} focused=${v.focused}`);


  // ── 4. desktop: a REAL mouse click on a hit navigates ──────────────────
  // The regression this pins (round 5 of #124): the results are teleported
  // outside the field's box, and a containment test in a `focusout` handler
  // called every hit "outside" — mousedown blurred the input, the panel
  // closed, and the anchor was gone before `click`. Only a real pointer
  // sequence FROM A FOCUSED FIELD (mousedown → blur → mouseup) reproduces
  // it; a synthetic `.click()` does not, and neither does an unfocused field.
  // Geometry re-read AFTER the re-query: the earlier `v` described a list that
  // the negative control has since replaced.
  const v2raw = await safe(`(() => {
    const hits = document.querySelectorAll('.lagrange-docs-search-hit');
    if (!hits.length) return JSON.stringify({ hits: 0, cx: 0, cy: 0, href: null, focused: document.activeElement?.className });
    const b = hits[0].getBoundingClientRect();
    return JSON.stringify({ hits: hits.length, cx: b.left + b.width / 2, cy: b.top + b.height / 2, href: hits[0].getAttribute('href'), focused: document.activeElement?.className });
  })()`);
  const v2 = v2raw ? JSON.parse(v2raw) : { hits: 0, cx: 0, cy: 0, href: null, focused: null };
  check("desktop: the re-query restored results for the click test", v2.hits > 0 && typeof v2.focused === "string" && v2.focused.includes("hk-search-input-field"), `hits=${v2.hits} focused=${v2.focused}`);
  const before = await safe(`location.pathname + location.hash`);
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: v2.cx, y: v2.cy });
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: v2.cx, y: v2.cy, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 60));
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: v2.cx, y: v2.cy, button: "left", clickCount: 1 });
  const navAfter = await poll(async () => {
    const a = await safe(`location.pathname + location.hash`);
    return a && a !== before ? a : null;
  }, 6);
  const after = navAfter ?? (await safe(`location.pathname + location.hash`));
  // Navigate = the location changed AND it is the hit's own target (the
  // href may carry a hash; the path must match).
  const hrefPath = (v2.href ?? "").split("#")[0];
  const navigated = !!v2.href && after !== before && after.startsWith(hrefPath);
  check("desktop: clicking a hit navigates to it", navigated, `before=${before} after=${after} href=${v2.href}`);

} finally {
  await closeTab();
}

if (failures.length) {
  console.error(`\n${failures.length} check(s) failed: ${failures.join("; ")}`);
  process.exit(1);
}
writeFileSync(OK_SENTINEL, new Date().toISOString());
console.log("\nall checks passed");
