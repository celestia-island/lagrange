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

import { existsSync, readFileSync, readdirSync } from "node:fs";
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

/** The declaration body of the first rule whose selector is EXACTLY
 *  `selector` (searched from `from`), or null when there is no such block.
 *
 *  An exact token match matters: a plain `indexOf` also matches
 *  `.lg-diagram-canvas` inside `.lg-diagram-canvas-x`, so a renamed selector
 *  would keep the guard green while the shipped sheet styles nothing
 *  (measured — that survived an earlier version of this file). Depth is
 *  tracked by BRACE COUNT, so one-line nested SCSS still reads correctly. */
function ruleBody(css: string, selector: string, from = 0): string | null {
  let idx = css.indexOf(selector, from);
  while (idx >= 0) {
    const before = idx === 0 ? "" : css[idx - 1];
    let open = idx + selector.length;
    while (open < css.length && /\s/.test(css[open])) open += 1;
    const startsSelector = before === "" || /[\s{};,]/.test(before);
    if (startsSelector && css[open] === "{") {
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
    idx = css.indexOf(selector, idx + 1);
  }
  return null;
}

const sheet = stripComments(
  readFileSync(resolve(here, "../docsContent.scss"), "utf8"),
);

/** The Rust renderer owns the markup this sheet styles. Read it so a rename
 *  on either side (a live mutation let `> .lg-diagram-canvas` become
 *  `-x` and still pass a source-shape-only pin) fails here — the guard must
 *  cover the seam, not just the sheet. */
const rendererPath = resolve(here, "../../../../../src/render.rs");
if (!existsSync(rendererPath)) {
  // A move of the renderer must fail HERE with a readable message, not as an
  // opaque ENOENT at import time.
  throw new Error(
    `diagramCardLayout guard: the Rust renderer moved (expected ${rendererPath}) — `
      + "point this cross-file read at its new home",
  );
}
const renderer = readFileSync(rendererPath, "utf8");

describe("diagram card layout", () => {
  it("keeps the canvas a stretched flex item with a real width floor", () => {
    const preview = ruleBody(sheet, ".lg-diagram-preview");
    expect(preview, ".lg-diagram-preview rule must exist").not.toBeNull();
    // The premise of the defect: a flex row centres/shrinks its items.
    expect(preview!).toMatch(/display:\s*flex/);

    const canvas = ruleBody(preview!, ".lg-diagram-canvas");
    expect(canvas, "the canvas rule must stay nested under the preview").not.toBeNull();
    // `flex: 1 1 auto` makes the item claim the row (so a `width: 100%` child
    // has a definite width to resolve against). `min-width: 0` is the standard
    // companion for a flex item that may carry wide content: it opts out of
    // the automatic min-content floor so the preview's `overflow-x` keeps
    // working. (Measured after this fix: the definite width already implies
    // the floor, so the declaration is defensive — it matters again if the
    // board ever reports a min-content size.)
    expect(canvas!).toMatch(/flex:\s*1 1 auto/);
    expect(canvas!).toMatch(/min-width:\s*0/);
  });

  it("styles the canvas class the renderer actually emits", () => {
    // The sheet and the markup are two files: a selector that no longer
    // matches the emitted class paints nothing while every other gate stays
    // green (measured: renaming the selector survived the shape pin alone).
    expect(renderer).toContain('attr("class", "lg-diagram-canvas")');
    expect(ruleBody(sheet, ".lg-diagram-canvas")).not.toBeNull();
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

describe("diagram embedded content spacing", () => {
  const legacy = stripComments(readFileSync(resolve(here, "../../../../../styles/layout.scss"), "utf8"));

  it("removes preview padding and source-card margins in both lanes", () => {
    // MUT:diagram-flush-spacing — restoring preview padding or the generic
    // 1rem margin inside a source pane catches the source-shape assertions in
    // each lane; ordinary fenced blocks keep their separate margin pins below.
    for (const [label, css] of [["webui", sheet], ["legacy", legacy]] as const) {
      expect(ruleBody(css, ".lg-diagram-preview"), label).toMatch(/padding:\s*0/);
      const source = ruleBody(css, ".lg-diagram-source");
      expect(source, `${label} source rule`).not.toBeNull();
      expect(source!).toMatch(/border-top:\s*0/);
      expect(ruleBody(source!, ".hk-code-highlight"), label).toMatch(/margin:\s*0/);
    }
  });

  it("caps the legacy diagram canvas at the same default viewport share", () => {
    const preview = ruleBody(legacy, ".lg-diagram-preview")!;
    expect(ruleBody(preview, ".lg-diagram-canvas")).toMatch(/max-height:\s*40vh/);
  });

  /// Hydration moves each pane's INNER HTML into `.lagrange-docs-diagram-pane`,
  /// so the server wrapper `.lg-diagram-source` is gone by the time a reader
  /// sees the card — the margin rule above stops matching. Measured live
  /// 2026-11-28: the source body kept `margin: 16px 0px`. This pins the
  /// hydrated selector, and the boot test pins the data attribute it needs.
  ///
  /// MUT:diagram-pane-identity — dropping either half re-introduces the gap.
  it("zeroes the source card margin in the HYDRATED pane too", () => {
    const rule = ruleBody(sheet, '.lagrange-docs-diagram-pane[data-pane="source"] .hk-code-highlight');
    expect(rule, "hydrated source selector must exist").not.toBeNull();
    expect(rule).toMatch(/margin:\s*0/);
  });
});

/**
 * Code blocks need breathing room. The `.hk-code-highlight*` rules the card
 * bridges in from hikari's component sheet give the box no margin, and the
 * generic `pre` rule in this sheet deliberately skips
 * `.hk-code-highlight-code` — so a fenced block sat flush against its
 * neighbours (the CLA page's `Signed-off-by:` block, docs.celestia.world
 * 2026-11-27). Both lanes carry the rule: this one for the webui shell,
 * `styles/layout.scss` for the legacy lane.
 *
 * MUT: deleting (or zeroing) the margin turns both pins RED.
 */
describe("code block margins", () => {
  const legacy = stripComments(readFileSync(resolve(here, "../../../../../styles/layout.scss"), "utf8"));

  // MUT: zeroing the margin rule below never passes — this pin catches it,
  // and the legacy-lane twin catches it again.
  it("gives every code block at least 1rem above and below (webui lane)", () => {
    const rule = ruleBody(sheet, ".hk-code-highlight");
    expect(rule, ".hk-code-highlight rule must exist under the body scope").not.toBeNull();
    expect(rule).toMatch(/margin:\s*1rem 0/);
  });

  /// The card header is LAGRANGE chrome and must be flexed by lagrange's own
  /// sheet: its old `hi-code-highlight-header` borrow only looked right because
  /// the aliased component sheet happened to flex it, and deleting the borrow
  /// without a local rule left the label and the pill stacked on two rows
  /// (measured live 2026-11-28: `display: block`, zero padding).
  ///
  /// MUT: dropping either `.lg-diagram-header` block never passes these.
  it("flexes the diagram header in both lanes", () => {
    for (const [label, css] of [
      ["webui", sheet],
      ["legacy", legacy],
    ] as const) {
      const rule = ruleBody(css, ".lg-diagram-header");
      expect(rule, `${label} sheet must style .lg-diagram-header`).not.toBeNull();
      expect(rule).toMatch(/display:\s*flex/);
      expect(rule).toMatch(/justify-content:\s*space-between/);
      // …and the language chip gets its own type treatment.
      expect(ruleBody(css, ".lg-diagram-lang")).toMatch(/text-transform:\s*uppercase/);
    }
  });

  /// The renderer emits the LIBRARY's classes only: `hi-code-highlight*` was
  /// the hand-written scaffold's namespace and the alias bridge that styled it
  /// is gone. A stray `hi-` rule here would be dead CSS — or, worse, a sign the
  /// markup went back to being hand-written.
  ///
  /// MUT: reintroducing a `hi-code-highlight` rule (or going back to hand-written
  /// markup) never passes these assertions.
  it("carries no retired hi- code-highlight rules in either lane", () => {
    for (const [label, css] of [
      ["webui", sheet],
      ["legacy", legacy],
    ] as const) {
      expect(
        css.includes(".hi-code-highlight"),
        `${label} sheet still styles the retired scaffold`,
      ).toBe(false);
      expect(
        css.includes(".hk-code-highlight"),
        `${label} sheet must style the component`,
      ).toBe(true);
    }
  });

  it("gives every code block at least 1rem above and below (legacy lane)", () => {
    const content = ruleBody(legacy, ".content")!;
    const rule = ruleBody(content, ".hk-code-highlight");
    expect(rule, "the rule must stay nested under .content").not.toBeNull();
    expect(rule).toMatch(/margin:\s*1rem 0/);
    // the legacy article is selectable too
    expect(content).toMatch(/user-select:\s*text/);
  });
});

/**
 * The parser's own `language-mermaid` fences, rendered through the SAME
 * highlighter lagrange uses at build time, must carry token spans.
 *
 * syntect ships no Mermaid syntax: before `src/highlight/mermaid.sublime-syntax`
 * a ```` ```mermaid ```` source pane held zero `<span>`s while every other
 * fence was fully coloured (docs.celestia.world 2026-11-27). This guard reads
 * the grammar and every corpus fence — not a hand-written sample — so a
 * future fence using a construct the grammar drops shows up HERE.
 */
describe("the mermaid corpus stays highlightable", () => {
  const grammar = resolve(here, "../../../../../src/highlight/mermaid.sublime-syntax");

  it("ships the grammar the renderer loads", () => {
    expect(existsSync(grammar)).toBe(true);
    const source = readFileSync(grammar, "utf8");
    // the spans the sheet colours: keyword / string / comment / constant
    for (const scope of ["keyword.operator.link.mermaid", "keyword.control.diagram.mermaid"]) {
      expect(source, `${scope} in the grammar`).toContain(scope);
    }
  });

  it("every mermaid fence in the docs tree opens with a head the grammar scopes", () => {
    const docsRoot = resolve(here, "../../../../../docs");
    const files: string[] = [];
    const walk = (dir: string): void => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const full = resolve(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else if (entry.name.endsWith(".md")) files.push(full);
      }
    };
    walk(docsRoot);
    expect(files.length).toBeGreaterThan(0);

    const state = readFileSync(grammar, "utf8");
    // The head vocabulary straight out of the grammar's own `head:` rule —
    // reading it from there is the point: a fence whose head the rule forgot
    // fails here instead of silently losing every colour.
    // The alternation lives inside the grammar's `head` rule; pull it out
    // by line (the `\s` / `\d` escapes make a single regex brittle).
    // The `- match:` line sits directly above its `scope:` line.
    const lines = state.split("\n");
    const scopeAt = lines.findIndex((line) => line.includes("keyword.control.diagram.mermaid"));
    const headLine = scopeAt > 0 ? lines[scopeAt - 1] : undefined;
    expect(headLine, "the grammar's head rule").toBeTruthy();
    // The alternation is the FIRST `( … )` group of the match line (the
    // stateDiagram version suffix `(?:-v\d+)?` comes after it). Extract it
    // textually — balanced-paren scan from the first `(` — because the
    // grammar's own regex uses constructs (`(?i:…)`), escapes and
    // backreference-free but Oniguruma-only syntax that a JS RegExp cannot
    // compile.
    // Skip the inline-flag prefix `(?i:` before the real group.
    const body = headLine!.slice(headLine!.indexOf("(?i:") + 4);
    const open = body.indexOf("(");
    expect(open, "the head rule opens a group").toBeGreaterThanOrEqual(0);
    let depth = 0;
    let close = -1;
    for (let i = open; i < body.length; i += 1) {
      if (body[i] === "(") depth += 1;
      else if (body[i] === ")") {
        depth -= 1;
        if (depth === 0) {
          close = i;
          break;
        }
      }
    }
    const alternation = body.slice(open + 1, close);
    // The stateDiagram entry carries its own version-suffix group
    // (`(?:-v\d+)?`); everything else is a bare word.
    const heads = alternation.split("|").map((h) => h.replace(/\(\?:.*$/, ""));
    const fences = files.flatMap((file) =>
      [...readFileSync(file, "utf8").matchAll(/```mermaid\n([\s\S]*?)```/g)].map((m) => ({
        file,
        body: m[1],
      })),
    );
    expect(fences.length).toBeGreaterThan(0);
    for (const fence of fences) {
      const first = fence.body.split(/\r?\n/).find((line) => line.trim() !== "")?.trim() ?? "";
      const head = /^([A-Za-z]+)/.exec(first)?.[1] ?? "";
      const known = heads.some((h) => h.toLowerCase() === head.toLowerCase() || head.toLowerCase().startsWith(h.toLowerCase()));
      expect(known, `${fence.file}: mermaid head "${head}" is not in the grammar's head rule`).toBe(true);
    }
  });
});
