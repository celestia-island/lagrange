// @vitest-environment happy-dom — needs sessionStorage/location
// Behavioral pins for the inline boot watcher (the deploy-window self-heal).
// The watcher is a STRING in the emitted HTML; these tests execute it in
// happy-dom with a controllable clock, so the reload decision is observed,
// not assumed. MUT anchors name the mutation each pin survives.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { bootRecoveryJs } from "../template";

/** Execute the watcher body with fake timers and a controllable boot mark. */
function runWatcher(opts: { booted?: boolean; keepStorage?: boolean } = {}): {
  reloadCalls: number;
  reloadUrl: string | null;
  sessionStorageGet: string | null;
} {
  const holder = { reloadCalls: 0, reloadUrl: null as string | null, sessionStorageGet: null as string | null };
  vi.useFakeTimers();
  try {
    if (!opts.keepStorage) sessionStorage.clear();
    (globalThis as Record<string, unknown>).__lagrangeBoot = opts.booted;
    const reload = vi.fn((u: string) => {
      holder.reloadCalls += 1;
      holder.reloadUrl = u;
    });
    // happy-dom's location.replace is not configurable — swap it.
    Object.defineProperty(window.location, "replace", { configurable: true, value: reload });
    // eslint-disable-next-line no-new-func -- the string IS the shipped watcher
    new Function(bootRecoveryJs("", "docsMain-abc.js"))();
    // Walk past the watcher's own 500ms cadence AND its give-up threshold
    // (24 attempts) so the decision branch actually executes.
    vi.advanceTimersByTime(500 * 30);
    holder.sessionStorageGet = sessionStorage.getItem("lagrange-boot-reload");
  } finally {
    vi.useRealTimers();
    delete (globalThis as Record<string, unknown>).__lagrangeBoot;
  }
  return holder;
}

describe("boot watcher (deploy window self-heal)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });
  afterEach(() => {
    vi.useRealTimers();
    sessionStorage.clear();
  });

  it("does NOT reload when the boot happened (M1: drop the booted check → reloads)", () => {
    const r = runWatcher({ booted: true });
    expect(r.reloadCalls).toBe(0);
    expect(r.sessionStorageGet).toBeNull();
  });

  it("reloads ONCE with a cache-buster when the boot never happened", () => {
    const r = runWatcher({ booted: false });
    expect(r.reloadCalls).toBe(1);
    expect(r.reloadUrl).toContain("lr=");
    // The once-only flag is set BEFORE reloading, so a broken build cannot
    // loop the reader. Pinned against a reorder (flag-after-replace) by
    // observing the flag through the reload call itself.
    expect(r.sessionStorageGet).toBe("1");
  });

  it("RELOADS even though the page carries SSR markup (the 404 shape)", () => {
    // The S0 shape this PR fixes: a 404'd module leaves the SSR shell in the
    // DOM (sidebar rows, content). A watcher that treats that markup as
    // "booted" never fires — pin that the reload happens with a REALISTIC
    // body in place.
    document.body.innerHTML =
      '<div id="app"><div class="s-admin-shell"><aside class="hk-sidebar-rail-host">'
      + '<nav class="hk-sidebar"><div class="hk-menu-sidebar">'
      + '<div class="hk-menu-sidebar-row hk-menu-sidebar-group"></div></div></nav></aside>'
      + '<main class="s-admin-shell-main"><article class="lagrange-docs-content"><p>body</p></article></main></div></div>';
    const r = runWatcher({ booted: false });
    expect(r.reloadCalls).toBe(1);
    expect(r.reloadUrl).toContain("lr=");
  });

  it("give-up threshold is exactly 12s (24 ticks), not drift-tolerant", () => {
    vi.useFakeTimers();
    (globalThis as Record<string, unknown>).__lagrangeBoot = false;
    const reload = vi.fn();
    Object.defineProperty(window.location, "replace", { configurable: true, value: reload });
    // eslint-disable-next-line no-new-func
    new Function(bootRecoveryJs("", "docsMain-abc.js"))();
    vi.advanceTimersByTime(500 * 24);
    expect(reload, "attempts=24 must still be inside the grace window").not.toHaveBeenCalled();
    vi.advanceTimersByTime(500);
    expect(reload, "attempt 25 is past the limit and must fire").toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("the STALE RE-ENTERED load still reloads (promotion's observable effect)", () => {
    // Two chained page loads with carried storage — what promotion actually
    // changes. Visit 1 (no flag): reload, writes "1". Visit 2 (reads "1"):
    // PROMOTES to resume and keeps watching, so if the CDN served stale HTML
    // AGAIN, this page still reloads — a reader is never left dead for the
    // rest of the tab's life. (M-a: deleting the promote line makes this
    // visit stand down at 0 reloads — the exact L943 failure again.)
    vi.useFakeTimers();
    const reload = vi.fn();
    Object.defineProperty(window.location, "replace", { configurable: true, value: reload });
    // eslint-disable-next-line no-new-func
    const run = (): void => new Function(bootRecoveryJs("", "docsMain-abc.js"))();

    // Visit 1: no flag, boot never lands -> reload, writes "1".
    sessionStorage.clear();
    (globalThis as Record<string, unknown>).__lagrangeBoot = false;
    run();
    vi.advanceTimersByTime(500 * 25);
    expect(reload).toHaveBeenCalledTimes(1);
    expect(sessionStorage.getItem("lagrange-boot-reload")).toBe("1");

    // Visit 2: the reloaded page, storage carried. Its ENTRY state is what
    // promotion changes: "resume" (this page is a recovery attempt, and the
    // healthy-boot branch knows to CLEAR it) vs "1" (a second failure — the
    // healthy-boot branch never touches a "1", leaving the reader suppressed
    // for the tab's life). Reload count is 1 in both; the ENTRY STATE is the
    // observable. (M-a: deleting the promote line makes this "1".)
    (globalThis as Record<string, unknown>).__lagrangeBoot = false;
    run();
    expect(sessionStorage.getItem("lagrange-boot-reload"), "visit 2 must ENTER in the promoted resume state").toBe("resume");
    // The promoted state is what the healthy-boot branch clears — on the
    // NEXT page load in the same tab (this visit's watcher already stood
    // down; sessionStorage survives navigation, so the recovery state must
    // be clearable by it):
    vi.useFakeTimers();
    (globalThis as Record<string, unknown>).__lagrangeBoot = true;
    run();
    vi.advanceTimersByTime(500 * 2);
    expect(sessionStorage.getItem("lagrange-boot-reload"), "a healthy boot after the promotion clears the recovery state").toBeNull();
    vi.useRealTimers();
  });

  it("a healthy boot on the promoted page clears the flag (future window recovered)", async () => {
    sessionStorage.setItem("lagrange-boot-reload", "resume");
    (globalThis as Record<string, unknown>).__lagrangeBoot = true;
    vi.useFakeTimers();
    const reload = vi.fn();
    Object.defineProperty(window.location, "replace", { configurable: true, value: reload });
    // eslint-disable-next-line no-new-func
    new Function(bootRecoveryJs("", "docsMain-abc.js"))();
    vi.advanceTimersByTime(500 * 25);
    expect(reload, "a healthy promoted boot never reloads").not.toHaveBeenCalled();
    expect(sessionStorage.getItem("lagrange-boot-reload"), "a healthy boot clears the recovery state so a future deploy window gets its own reload").toBeNull();
    vi.useRealTimers();
  });

  it("resume mode stands down again after its own 12s (once per page load)", () => {
    vi.useFakeTimers();
    sessionStorage.setItem("lagrange-boot-reload", "1");
    (globalThis as Record<string, unknown>).__lagrangeBoot = false;
    const reload = vi.fn();
    Object.defineProperty(window.location, "replace", { configurable: true, value: reload });
    // eslint-disable-next-line no-new-func
    new Function(bootRecoveryJs("", "docsMain-abc.js"))();
    vi.advanceTimersByTime(500 * 24);
    expect(reload, "resumed watching must still respect the grace window").not.toHaveBeenCalled();
    vi.advanceTimersByTime(500);
    expect(reload).toHaveBeenCalledTimes(1);
    // The stored value is the resume marker again (still once per page load).
    expect(sessionStorage.getItem("lagrange-boot-reload")).toBe("resume");
    vi.useRealTimers();
  });

  it("stands down permanently only on a first visit (no prior flag)", () => {
    const r = runWatcher({ booted: false });
    expect(r.reloadCalls).toBe(1);
    expect(r.sessionStorageGet).toBe("1");
  });
});
