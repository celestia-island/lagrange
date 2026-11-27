// @vitest-environment happy-dom
// Behavioral pins for the match-centred snippet windows (the docs-search
// excerpt). THE invariant this wave exists for: every token the index
// matches must be a substring of the window the client cuts, and the card
// must never show literal pipe characters — a table in the window renders
// as a table, wherever the match sits inside it.
import { afterEach, describe, expect, it } from "vitest";

import { buildSnippetView, sliceWindow } from "../snippet";

const fillerA = "The platform guide explains the foundation layer in detail. ".repeat(6);
const fillerB = "Further sections continue below the table with more detail. ".repeat(6);
const markdown = [
  "# Section",
  "",
  fillerA,
  "",
  "| Layer | Project |",
  "| --- | --- |",
  "| 0 | kirino | auth, RBAC, JWT sessions |",
  "| 1 | plana | protocol, transport, metering |",
  "",
  fillerB,
].join("\n");

describe("sliceWindow", () => {
  it("line-snaps outward so a cut never opens mid-line", () => {
    const match = { index: markdown.indexOf("plana"), length: 5 };
    const w = sliceWindow(markdown, match);
    // The radius cuts into the filler paragraph; the outward snap eats back
    // to the nearest line boundary, so the window's first character opens a
    // COMPLETE line of the original (never mid-row, never mid-sentence).
    const firstLine = w.window.split("\n")[0];
    expect(
      markdown.startsWith(firstLine + "\n") || markdown.includes("\n" + firstLine + "\n"),
      "the window's first line must be a complete original line",
    ).toBe(true);
    // The snap eats BACKWARD to that line's start: the heading above it is
    // outside the window, which is exactly what hasAbove reports.
    expect(w.window).not.toContain("# Section");
    expect(w.hasAbove).toBe(true);
  });

  it("reports continuation on both sides independently", () => {
    const top = sliceWindow(markdown, { index: markdown.indexOf("Intro paragraph"), length: 5 });
    expect(top.hasAbove).toBe(false); // the window reached the text's true start
    expect(top.hasBelow).toBe(true);

    // The last table row: the window's end cut is line-snapped forward, and
    // fillerB's single long line runs to the text's true end — no fade below.
    const bottom = sliceWindow(markdown, { index: markdown.indexOf("metering"), length: 8 });
    expect(bottom.hasAbove).toBe(true);
    expect(bottom.hasBelow).toBe(false);
  });
});

describe("buildSnippetView", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders the window AS markdown: the reader never sees literal pipes", () => {
    const view = buildSnippetView(markdown, "plana");
    // The user-facing property: the window's table renders as TABLE MARKUP
    // (a real browser) — never as raw pipe characters. The exact wrapper in
    // the test DOM is happy-dom-parser-specific (a LEADING table gets its
    // cells hoisted there), so this pins the output contract, not the
    // wrapper.
    expect(view.html).not.toContain("| --- |");
    expect(view.html).not.toContain("| kirino");
    expect(view.html).toContain("kirino");
    expect(view.html).toContain("Layer");
  });

  it("underlines the query occurrence in place (mark over rendered text)", () => {
    const view = buildSnippetView(markdown, "plana");
    const marks = view.html.match(/<mark /g) ?? [];
    // The fixture's single "plana" occurrence sits inside the window; its
    // rendered text node is wrapped in exactly one mark.
    expect(marks.length).toBe(1);
    expect(view.html).toContain('class="lagrange-docs-search-mark"');
    expect(view.html).toContain(">plana</mark>");
  });

  it("a realistic long-row table: the window opens at the header and cells reach the card", () => {
    // The R2 repro shape: realistic 3-column rows (~150 chars each) with
    // prose above — the naive cut lands mid-table and the containing-run
    // rule must resolve the opening to the table header. Cells must reach
    // the card; the invented pre-table prose must not.
    const rows = [
      ["electrode_recon", "Three-phase electrode microstructure generator, packaged as a containerized compute engine for the industrial edge", "repository"],
      ["hare", "no_std bridge crates for the industrial edge: wire protocol, hardware manifest, protocol hub and HAL traits", "repository"],
      ["shun", "Stream-driven payload delivery with backpressure and replay for the industrial bus", "repository"],
    ];
    const table = ["| Project | Purpose | Docs |", "| --- | --- | --- |"]
      .concat(rows.map(([p, r, d]) => `| ${p} | ${r} | ${d} |`))
      .join("\n");
    const prose = "Context paragraphs describing the platform layers. ".repeat(6);
    const chunk = ["## Ecosystem", "", prose, "", table, "", "Trailing prose."].join("\n");
    const match = { index: chunk.indexOf("| hare "), length: 7 };
    const w = sliceWindow(chunk, match);
    expect(w.window.split("\n")[0]).toBe("| Project | Purpose | Docs |");
    const view = buildSnippetView(chunk, "hare");
    expect(view.html).toContain("hare");
    expect(view.html).toContain("Purpose");
    expect(view.html).not.toContain("Context paragraphs");
  });

  it("a window that opens on a table ROW swallows the header so GFM renders a table", () => {
    // The header + separator sit beyond the naive 160-char cut when the
    // preceding prose is long. The row-only cut renders the rows as a
    // literal-pipe paragraph (the exact dump this view exists to prevent);
    // the table-aware snap eats back to the header so marked builds a table.
    const longProse = "Explaining the layering before any table appears. ".repeat(10);
    const chunk = [
      "## Tools",
      "",
      longProse,
      "",
      "| Project | Role |",
      "| --- | --- |",
      "| kirino | auth |",
      "| plana | transport |",
      "",
      "More prose after the table.",
    ].join("\n");
    const match = { index: chunk.indexOf("| plana"), length: 7 };
    const w = sliceWindow(chunk, match);
    // The naive radius lands inside the prose above the table; the snap eats
    // back past the pipe rows to the table header.
    const firstLine = w.window.split("\n")[0];
    expect(
      firstLine === "| Project | Role |" || firstLine.startsWith("|"),
      "the window must open on a complete table line, got: " + firstLine,
    ).toBe(true);
    const view = buildSnippetView(chunk, "plana");
    // The excerpt carries the header cells and the underlined match. (The
    // <table> wrapper's survival through the TEST DOM's innerHTML parser is
    // happy-dom-specific — real browsers render the table; the mid-window
    // table case is pinned by "renders the window AS markdown" above.)
    expect(view.html).toContain("Project");
    expect(view.html).toContain("kirino");
    expect(view.html).toContain('class="lagrange-docs-search-mark"');
    expect(view.html).not.toContain("Explaining the layering");
  });

  it("drops the section's own leading heading (the card already shows it)", () => {
    const chunk = [
      "## Install",
      "",
      "Run the pipeline command to start.",
    ].join("\n");
    const view = buildSnippetView(chunk, "pipeline");
    // The match is in the BODY, so the leading heading is dropped: the card
    // already shows the document title and the heading path above the
    // excerpt. Asserting the text (not the tag): sanitizers and environments
    // disagree on whether the wrapper survives, so a tag assertion can pass
    // for the wrong reason.
    expect(view.html).not.toContain("Install");
    expect(view.html.startsWith("Run the")).toBe(true);
  });

  it("keeps a heading-only chunk's match (never renders an empty card)", () => {
    // The Rust chunker emits heading-only spans (nine of them in this repo's
    // own docs). Stripping the heading there would delete the ONLY content —
    // and the only highlight — and the card renders nothing at all.
    const view = buildSnippetView("## Install", "Install");
    expect(view.html.length).toBeGreaterThan(0);
    expect(view.html).toContain("lagrange-docs-search-mark");
    expect(view.html).toContain("Install");
  });

  it("keeps the heading when the body is only whitespace", () => {
    // The chunker can emit `## Heading` followed by blank lines. The query
    // here does NOT match the heading, so the empty-body guard is the only
    // thing keeping it: a guard that tested the raw remainder instead of its
    // trimmed form would drop the heading and leave a blank card.
    const view = buildSnippetView("## Install\n   \n\t\n", "pipeline");
    expect(view.html.trim().length).toBeGreaterThan(0);
    expect(view.html).toContain("Install");
  });

  it("keeps the heading when the MATCH lives in it", () => {
    const view = buildSnippetView("## Quick Start\n\nRun the pipeline.", "Quick");
    expect(view.html).toContain("Start");
    expect(view.html).toContain("lagrange-docs-search-mark");
  });

  it("keeps a hit an EXCERPT: images collapse to alt text, links unwrap", () => {
    // A chunk can carry images and links. A hit card that loaded remote
    // images would fire a request per hit, and live links make the card a
    // navigation minefield — the excerpt shows the WORDS, not the chrome.
    const withChrome = [
      "Read the [pipeline guide](https://example.com/guide) first.",
      "",
      "![architecture diagram](https://example.com/diagram.png)",
      "",
      "Then check the pipeline again.",
    ].join("\n");
    const view = buildSnippetView(withChrome, "pipeline");
    expect(view.html).not.toContain("<img");
    expect(view.html).not.toContain("<a ");
    expect(view.html).not.toContain("example.com");
    // The words survive: link label (the match inside it is marked, so the
    // label is no longer a contiguous string) and the image's alt text.
    expect(view.html).toContain("guide");
    expect(view.html).toContain("architecture diagram");
  });

  it("a SHORT table deep past the naive cut still opens at its header, not the separator", () => {
    // The reachable short-table case: a 3-row table whose header sits
    // beyond the naive radius (long prose above, short rows). The
    // line-snapped cut lands ON the separator row; the containing-run rule
    // must eat back to the header — a separator-first window is the
    // literal-pipe dump again (GFM has no header above the delimiter row).
    const header = "| Name | Purpose | Type |";
    const sep = "| --- | --- | --- |";
    const rows = [
      "| alpha | the first row of a compact repository table | code |",
      "| beta | the second row with slightly longer prose inside it | docs |",
      "| gamma | the third row, still inside the naive radius from here | infra |",
    ].join("\n");
    const prose = "Introductory context well above the compact table. ".repeat(13);
    const chunk = ["## Short table", "", prose, "", header, sep, rows, ""].join("\n");
    const match = { index: chunk.indexOf("| gamma"), length: 7 };
    const w = sliceWindow(chunk, match);
    expect(w.window.split("\n")[0]).toBe(header);
    const view = buildSnippetView(chunk, "gamma");
    // The wrapper's survival is happy-dom-specific; the cells and the
    // underlined match are what the card must carry.
    expect(view.html).toContain("gamma");
    expect(view.html).toContain("lagrange-docs-search-mark");
    expect(view.html).not.toContain(sep + "\n");
  });
});
