// @vitest-environment happy-dom
// DPI prefs store (ported from chest with the lagrange storage key): the
// contracts the docs-shell block depends on — persisted-notch bounds,
// risky-scale detection, the preview→countdown→keep/revert lifecycle and
// the two UI-independent escapes (hard reset; notification). MUT anchors
// name the behavior each pin survives.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DPI_MAX,
  DPI_MIN,
  applyDpiPrefs,
  initDpiPrefs,
  isDpiCountdownActive,
  isDpiRisky,
  keepDpiScale,
  loadDpiScale,
  onDpiPrefsChange,
  previewDpiScale,
  resetAppliedDpiScaleForTest,
  resetDpiScale,
  revertPreviewDpiScale,
  saveDpiScale,
  shutdownDpiPrefs,
  useDpiCountdown,
} from "../dpiPrefs";

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-dpi-scale");
  document.documentElement.style.removeProperty("zoom");
  initDpiPrefs();
  vi.useFakeTimers();
});

afterEach(() => {
  shutdownDpiPrefs();
  vi.useRealTimers();
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-dpi-scale");
  document.documentElement.style.removeProperty("zoom");
  resetAppliedDpiScaleForTest();
});

describe("dpiPrefs", () => {
  it("defaults to Auto (null) and never touches the root", () => {
    expect(loadDpiScale()).toBeNull();
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("");
    expect(document.documentElement.hasAttribute("data-dpi-scale")).toBe(false);
  });

  it("bounds a persisted value: out-of-range notches read as Auto", () => {
    saveDpiScale(150);
    expect(loadDpiScale()).toBe(150);
    saveDpiScale(50);
    expect(loadDpiScale()).toBeNull();
    saveDpiScale(DPI_MAX + 25);
    expect(loadDpiScale()).toBeNull();
    expect(DPI_MIN).toBe(100);
  });

  it("applies the persisted scale as the root's zoom, Auto clears it", () => {
    saveDpiScale(125);
    applyDpiPrefs();
    expect(document.documentElement.dataset.dpiScale).toBe("125");
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.25");
    saveDpiScale(null);
    applyDpiPrefs();
    expect(document.documentElement.hasAttribute("data-dpi-scale")).toBe(false);
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("");
  });

  it("flags scales whose effective layout width falls under the usable floor", () => {
    // 300% on a 900px window lays out at 300px — under the 360px floor.
    expect(isDpiRisky(300, 900)).toBe(true);
    // 200% on the same window lays out at 450px — fine.
    expect(isDpiRisky(200, 900)).toBe(false);
    expect(isDpiRisky(0, 900)).toBe(false);
  });

  it("previews without persisting, keeps on answer, reverts on expiry", async () => {
    const seen: Array<number | null> = [];
    const un = onDpiPrefsChange(() => seen.push(loadDpiScale()));

    saveDpiScale(100);
    previewDpiScale(200);
    // Applied visually, NOT persisted; the countdown is live.
    expect(document.documentElement.dataset.dpiScale).toBe("200");
    expect(loadDpiScale()).toBe(100);
    expect(isDpiCountdownActive()).toBe(true);
    expect(useDpiCountdown().scale).toBe(200);

    keepDpiScale();
    expect(loadDpiScale()).toBe(200);
    expect(isDpiCountdownActive()).toBe(false);
    // keepDpiScale does not notify (the visual state did not move).

    previewDpiScale(300);
    await vi.advanceTimersByTimeAsync(10_000);
    // The safety net expired: back to the persisted value, countdown over.
    expect(isDpiCountdownActive()).toBe(false);
    expect(document.documentElement.dataset.dpiScale).toBe("200");
    // Every painted write notified, and each notification read the
    // PERSISTED value at that moment (preview writes while the stored
    // notch was 100, the expiry revert after 保存 had landed 200).
    expect(seen).toEqual([100, 200, 200]);
    un();
  });

  it("hard reset clears the persisted scale AND any live preview", () => {
    saveDpiScale(150);
    previewDpiScale(300);
    resetDpiScale();
    expect(loadDpiScale()).toBeNull();
    expect(isDpiCountdownActive()).toBe(false);
    expect(document.documentElement.hasAttribute("data-dpi-scale")).toBe(false);
  });

  it("revertPreview restores the persisted value without touching storage", () => {
    saveDpiScale(150);
    previewDpiScale(250);
    revertPreviewDpiScale();
    expect(loadDpiScale()).toBe(150);
    expect(document.documentElement.dataset.dpiScale).toBe("150");
  });

  it("boot rollback clears a persisted scale too risky for this viewport", () => {
    // 300% on a 900px window is risky at boot — cleared before first paint.
    shutdownDpiPrefs();
    saveDpiScale(300);
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 900 });
    initDpiPrefs();
    expect(loadDpiScale()).toBeNull();
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 1024 });
  });

  it("the ?dpi=auto URL hatch clears the persisted scale at boot", () => {
    shutdownDpiPrefs();
    saveDpiScale(150);
    window.history.replaceState(null, "", "/?dpi=auto");
    initDpiPrefs();
    expect(loadDpiScale()).toBeNull();
    window.history.replaceState(null, "", "/");
  });
});
