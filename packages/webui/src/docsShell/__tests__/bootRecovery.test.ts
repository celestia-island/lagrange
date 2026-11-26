// Behavioral pins for the inline boot watcher (the deploy-window self-heal).
// The watcher is a STRING in the emitted HTML; these tests execute it in
// happy-dom with a controllable clock, so the reload decision is observed,
// not assumed. MUT anchors name the mutation each pin survives.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
// @vitest-environment happy-dom — needs sessionStorage/location

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
    // loop the reader.
    expect(r.sessionStorageGet).toBe("1");
  });

  it("stands down on a second visit (sessionStorage flag set)", () => {
    sessionStorage.setItem("lagrange-boot-reload", "1");
    const r = runWatcher({ keepStorage: true });
    expect(r.reloadCalls).toBe(0);
    // And the visit leaves the flag untouched (the reader is not stuck in a
    // "suppressed forever" state that a later good deploy could not clear).
    expect(r.sessionStorageGet).toBe("1");
  });
});
