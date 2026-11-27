// @vitest-environment happy-dom
// The collapse rule, pinned on the EXACT chunk that showed the bug.
//
// 2026-11-27 field report (phone screenshot): searching 测试 on
// docs.celestia.world showed a 闭环 hit whose card was a five-row table
// with NO underline at all, 267px tall beside 75px neighbours. The chunk's
// match sits at char 1322 of 1534 — table-header snapping moved the window
// opening to char 9, so the mark landed ~1300 chars below a 13rem clamp and
// the card rendered its own match out of sight.
//
// The fixture below is the REAL shipped text of that chunk, trimmed to the
// rows that matter. It is deliberately not a synthetic one-liner: the bug
// lives in real table arithmetic.
import { describe, expect, it } from "vitest";

import { CONTEXT_CHARS, buildExcerptLines, needsCollapse, toPlainText } from "../collapse";
import { buildSnippetView } from "../snippet";

const TABLE_CHUNK = [
  "## 逐环分解",
  "",
  "| # | 环节 | 发生什么 | 项目 |",
  "| --- | --- | --- | --- |",
  "| 1 | **发现** | 潜在用户找到生态、理解其哲学并选择入口 | [docs.celestia.world](https://docs.celestia.world)（本站） |",
  "| 2 | **安装** | 用户得到可运行的系统：管理面板、桌面/Web 外壳 | [arona](https://github.com/celestia-island/arona)（云 API 管理面板） |",
  "| 7 | **验证与支撑** | 真机集成测试、监督与自愈、用量记录、反馈通道 | [celestia-integration](https://github.com/celestia-island/celestia-integration) |",
].join("\n");

const PROSE_CHUNK = [
  "## 闭环如何运转",
  "",
  "- **每一步都可测试。** 每个环节在 celestia-integration 中有对应的验收测试；直到整个闭环在真实节点上通过，发布才算绿。",
].join("\n");

describe("needsCollapse (自动分析正文)", () => {
  it("flags table, list and fence windows", () => {
    expect(needsCollapse("| a | b |\n| --- | --- |\n| 1 | 2 |")).toBe(true);
    expect(needsCollapse("- one\n- two")).toBe(true);
    expect(needsCollapse("1. one\n2. two")).toBe(true);
    expect(needsCollapse("```\ncode\n```")).toBe(true);
  });

  it("leaves prose alone (it keeps its markdown render)", () => {
    expect(needsCollapse("The **plana** layer carries transport.")).toBe(false);
    expect(needsCollapse("## Heading\n\nA plain paragraph with no block structure.")).toBe(false);
  });
});

describe("toPlainText", () => {
  it("drops block syntax but keeps the words in reading order", () => {
    const flat = toPlainText(TABLE_CHUNK);
    expect(flat).not.toContain("|");
    expect(flat).not.toContain("---");
    expect(flat).not.toContain("##");
    expect(flat).not.toContain("**");
    // A link keeps its label and drops its target.
    expect(flat).toContain("arona");
    expect(flat).not.toContain("github.com");
    // Block order survives: heading, then the first row's first cell.
    expect(flat.indexOf("逐环分解")).toBeLessThan(flat.indexOf("真机集成测试"));
  });
});

describe("buildExcerptLines — the six-character rule", () => {
  it("keeps at most CONTEXT_CHARS each side of the match", () => {
    const lines = buildExcerptLines(TABLE_CHUNK, ["测试"]);
    expect(lines.length).toBeGreaterThan(0);
    const segments = lines[0].segments;
    const idx = segments.findIndex((s) => s.match);
    expect(idx, "the excerpt must contain the match").toBeGreaterThanOrEqual(0);
    const before = segments.slice(0, idx).filter((s) => s.text !== "…");
    const after = segments.slice(idx + 1).filter((s) => s.text !== "…");
    for (const seg of before) {
      expect(Array.from(seg.text).length).toBeLessThanOrEqual(CONTEXT_CHARS);
    }
    for (const seg of after) {
      expect(Array.from(seg.text).length).toBeLessThanOrEqual(CONTEXT_CHARS);
    }
  });

  it("THE regression: the table chunk's excerpt shows the match, not row 1", () => {
    // Before the fix the rendered card kept the table's OPENING and lost the
    // match entirely. The excerpt must lead with the match's neighbourhood.
    const lines = buildExcerptLines(TABLE_CHUNK, ["测试"]);
    const plain = lines[0].segments.map((s) => s.text).join("");
    expect(plain).toContain("测试");
    expect(plain).toContain("真机集成");
    // The first table row is nowhere near the match and must not survive.
    expect(plain).not.toContain("潜在用户找到生态");
  });

  it("emits the ellipsis only on a side that really dropped text", () => {
    // A match at the very start has nothing above it — a leading "…" would
    // imply content that never existed.
    const atStart = buildExcerptLines("测试位于开头，后面还有很多内容需要省略。", ["测试"]);
    expect(atStart[0].segments[0]).toEqual({ text: "测试", match: true });
    // A match in the middle elides on both sides.
    const middle = buildExcerptLines("前缀文字很多很多测试后缀文字也很多很多", ["测试"]);
    const texts = middle[0].segments.map((s) => s.text);
    expect(texts[0]).toBe("…");
    expect(texts[texts.length - 1]).toBe("…");
  });

  it("shows at most two matches, so a row never grows into a list", () => {
    const many = Array.from({ length: 6 }, (_, i) => `第${i}段 测试 出现在这里。`).join(" ");
    const lines = buildExcerptLines(many, ["测试"]);
    expect(lines.length).toBeLessThanOrEqual(2);
  });

  it("never returns raw markdown punctuation to the card", () => {
    for (const line of buildExcerptLines(TABLE_CHUNK, ["测试"])) {
      const plain = line.segments.map((s) => s.text).join("");
      expect(plain).not.toContain("|");
      expect(plain).not.toContain("**");
      expect(plain).not.toContain("](");
    }
  });
});

describe("buildSnippetView — uniform rows", () => {
  it("collapses the table chunk and marks its match (the missing-underline bug)", () => {
    const view = buildSnippetView(TABLE_CHUNK, "测试");
    expect(view.compacted).toBe(true);
    expect(view.html).toContain(">测试</mark>");
    // No table markup at all: the row is one bounded line per match.
    expect(view.html).not.toContain("<table");
    expect(view.html).not.toContain("<td");
  });

  it("keeps prose rendered, and a bullet list collapses (it cannot be squeezed)", () => {
    const prose = "The **plana** layer carries transport and metering.";
    expect(buildSnippetView(prose, "plana").compacted).toBe(false);

    // A list is block structure: N bullets stay N bullets however short the
    // cells get, so it takes the flattened path.
    const listed = buildSnippetView(PROSE_CHUNK, "测试");
    expect(listed.compacted).toBe(true);
    expect(listed.html).toContain(">测试</mark>");
  });
});