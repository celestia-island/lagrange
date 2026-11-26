// @vitest-environment happy-dom — the builder walks the rendered DOM
// Match-centred snippet windows (L942 follow-up) — the builder is pure over
// strings + the DOM walk, so the pins observe exactly what the hit card
// renders: the window's line snapping, the directional fade flags, the
// markdown rendering (pipe rows become tables), and the in-place <mark>ing.
import { afterEach, describe, expect, it } from "vitest";

import { buildSnippetView, sliceWindow } from "../snippet";

const fillerA = "The platform guide explains the foundation layer in detail. ".repeat(6);
const fillerB = "Further sections continue below the table with more detail. ".repeat(6);
const markdown = [
  "# Section",
  "",
  fillerA,
  "",
  "| Layer | Project | Role |",
  "| --- | --- | --- |",
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

  it("renders the window AS markdown: pipe rows become a real table", () => {
    const view = buildSnippetView(markdown, "plana");
    expect(view.html).toContain("<table>");
    expect(view.html).toContain("<td>");
    expect(view.html).not.toContain("| --- |");
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

  it("drops the section's own leading heading (the card already shows it)", () => {
    const chunk = [
      "## Install",
      "",
      "Run the pipeline command to start.",
    ].join("\n");
    const view = buildSnippetView(chunk, "pipeline");
    // The heading text is GONE (asserting the text, not the tag: sanitizers
    // and environments disagree on whether the wrapper survives, so a tag
    // assertion can pass for the wrong reason).
    expect(view.html).not.toContain("Install");
    expect(view.html.startsWith("Run the")).toBe(true);
  });

  it("keeps a heading-only chunk's match (never renders an empty card)", () => {
    // The Rust chunker emits heading-only spans (9 of them in this repo's
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
    const view = buildSnippetView(["## Quick Start", "", "Run the pipeline."].join("\n"), "Quick");
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
});
