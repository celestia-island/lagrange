// @vitest-environment happy-dom — the upgrade walks the DOM and reads layout
import { beforeEach, describe, expect, it } from "vitest";

import { mountTableScrollShells } from "../docsMain";

/** Give a host a fake overflow geometry: happy-dom does not lay out, so the
 *  three measured numbers are stubbed directly. */
function stubGeometry(host: HTMLElement, g: { scrollLeft: number; clientWidth: number; scrollWidth: number }): void {
  Object.defineProperty(host, "scrollLeft", { configurable: true, writable: true, value: g.scrollLeft });
  Object.defineProperty(host, "clientWidth", { configurable: true, writable: true, value: g.clientWidth });
  Object.defineProperty(host, "scrollWidth", { configurable: true, writable: true, value: g.scrollWidth });
}

function bandEdges(host: HTMLElement): string[] {
  return [...host.querySelectorAll<HTMLElement>(".hk-edge-glow-band")].map(
    (b) => b.dataset.edge ?? "",
  );
}

describe("mountTableScrollShells", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("mounts ONE painter per host (idempotent across repeated upgrades)", () => {
    document.body.innerHTML =
      '<div class="lagrange-docs-table-scroll"><table><tbody><tr><td>x</td></tr></tbody></table></div>';
    const host = document.querySelector<HTMLElement>(".lagrange-docs-table-scroll")!;
    stubGeometry(host, { scrollLeft: 0, clientWidth: 300, scrollWidth: 900 });

    mountTableScrollShells(document);
    mountTableScrollShells(document);
    mountTableScrollShells(document);

    expect(host.querySelectorAll(".hk-edge-glow").length).toBe(1);
  });

  it("lights only the edges that hide content (both mid-scroll, none when parked)", async () => {
    document.body.innerHTML =
      '<div class="lagrange-docs-table-scroll"><table><tbody><tr><td>x</td></tr></tbody></table></div>';
    const host = document.querySelector<HTMLElement>(".lagrange-docs-table-scroll")!;

    // Parked at the START of a wide table: only the RIGHT edge hides content.
    stubGeometry(host, { scrollLeft: 0, clientWidth: 300, scrollWidth: 900 });
    mountTableScrollShells(document);
    expect(bandEdges(host)).toEqual(["right"]);

    // MID-scroll: content hides on BOTH sides, so both bands must exist.
    // The sense runs on the next animation frame after a scroll event, so
    // the assertion waits one frame — driving the real path instead of
    // re-calling the (guarded) entry point.
    stubGeometry(host, { scrollLeft: 300, clientWidth: 300, scrollWidth: 900 });
    host.dispatchEvent(new Event("scroll"));
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    expect(bandEdges(host)).toEqual(["left", "right"]);

    // Parked at the END: nothing hides to the right any more.
    stubGeometry(host, { scrollLeft: 600, clientWidth: 300, scrollWidth: 900 });
    host.dispatchEvent(new Event("scroll"));
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    expect(bandEdges(host)).toEqual(["left"]);
  });

  it("yields NO bands for a table that fits (no lamp on a table that does not scroll)", () => {
    document.body.innerHTML =
      '<div class="lagrange-docs-table-scroll"><table><tbody><tr><td>x</td></tr></tbody></table></div>';
    const host = document.querySelector<HTMLElement>(".lagrange-docs-table-scroll")!;
    stubGeometry(host, { scrollLeft: 0, clientWidth: 600, scrollWidth: 600 });
    mountTableScrollShells(document);
    expect(bandEdges(host)).toEqual([]);
    // The painter root still mounts (its contract: an empty set keeps the
    // same subtree instead of mounting/unmounting).
    expect(host.querySelectorAll(".hk-edge-glow").length).toBe(1);
  });
});