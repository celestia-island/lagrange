// Diagram card layout guard: the canvas must keep claiming its flex row.
//
// The 2026-11-26 hub defect was invisible to every gate: `.lg-diagram-preview`
// is a flex row, `.lg-diagram-canvas` was a flex item with the default
// `flex: 0 1 auto`, and its content (hikari's mermaid board) is a
// `width: 100%` block — so the item had no intrinsic width, the percentage
// resolved against zero, the canvas collapsed to its 2px border, and the board
// fitted against a 0-wide viewport (painting the diagram mirrored and
// invisible). `vue-tsc`, the vitest suite and the docs build all stayed green;
// only a browser showed it. This pins the SOURCE shape that makes the card
// work, the same way `src/sheetShape.guard.test.ts` pins the console sheets.
//
// The legacy lane's twin (`styles/layout.scss`) deliberately keeps
// shrink-to-fit: its canvas holds a mermaid.js SVG with a real intrinsic
// width, so nothing collapses there — this guard covers the webui lane only.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));

/** Sheet source with comments removed, so a documented example can never
 *  satisfy a pin. */
function stripComments(css: string): string {
  return css
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:])\/\/[^\n]*/g, "$1");
}

/** The declaration body of the first rule whose selector text is `selector`
 *  (searched from `from`), or null when there is no such block. Depth is
 *  tracked by BRACE COUNT — SCSS may put a nested rule on one line. */
function ruleBody(
  css: string,
  selector: string,
  from = 0,
): string | null {
  const idx = css.indexOf(selector, from);
  if (idx < 0) return null;
  const open = css.indexOf("{", idx + selector.length);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < css.length; i += 1) {
    const ch = css[i];
    if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  return null;
}

const sheet = stripComments(
  readFileSync(resolve(here, "../docsContent.scss"), "utf8"),
);

describe("diagram card layout", () => {
  it("keeps the canvas a stretched flex item with a real width floor", () => {
    const preview = ruleBody(sheet, ".lg-diagram-preview");
    expect(preview, ".lg-diagram-preview rule must exist").not.toBeNull();
    // The premise of the defect: a flex row centres/shrinks its items.
    expect(preview!).toMatch(/display:\s*flex/);

    const canvas = ruleBody(preview!, ".lg-diagram-canvas");
    expect(canvas, "the canvas rule must stay nested under the preview").not.toBeNull();
    // `flex: 1 1 auto` makes the item claim the row (so a `width: 100%` child
    // has a definite width to resolve against); `min-width: 0` keeps that
    // from turning into a min-content floor that kills `overflow-x`.
    expect(canvas!).toMatch(/flex:\s*1 1 auto/);
    expect(canvas!).toMatch(/min-width:\s*0/);
  });

  it("the reader is not vacuous (negative control)", () => {
    // A selector that exists nowhere must read as absent — otherwise the pin
    // above could be satisfied by a stray match and the guard would be a
    // no-op on any real edit.
    expect(ruleBody(sheet, ".lg-diagram-canvas-that-does-not-exist")).toBeNull();
    // And the reader must actually descend into a nested block.
    const preview = ruleBody(sheet, ".lg-diagram-preview")!;
    expect(ruleBody(preview, ".katex")).toMatch(/color:/);
  });
});