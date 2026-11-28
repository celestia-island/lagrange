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
  it("keeps the word between spaced comparison operators (not a tag)", () => {
    // The R2 review's S1: `a < b > c` read `< b >` as an HTML tag and
    // DELETED the b. Only a letter right after `<` opens a tag.
    expect(toPlainText("The guard runs when a < b > c is true")).toContain("when a < b > c is true");
    expect(toPlainText("if low < mid > high then swap")).toContain("low < mid > high");
    // Real tags still go.
    expect(toPlainText("a <b>bold</b> c")).toBe("a bold c");
  });

  it("keeps snake_case identifiers searchable", () => {
    // The R2 review's S2: the emphasis stripper deleted EVERY underscore,
    // so `use_modal_state` flattened to `usemodalstate`. Lone underscores
    // are content (GFM agrees — intraword `_` never emphasizes).
    expect(toPlainText("call `use_modal_state` here")).toContain("use_modal_state");
    expect(toPlainText("**bold** and __strong__ stay")).toContain("bold and strong stay");
    // A paired-underscore emphasis at word boundaries still unwraps.
    expect(toPlainText("this _word_ goes")).toBe("this word goes");
  });

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

  it("keeps a BALANCED target out of the excerpt (GFM href, not first paren)", () => {
    // L962/②: the old `[^)\n]*` target stopped at the FIRST `)`, so a href
    // that carries its own parens (a wikipedia/disambiguation URL, an API
    // page, a localised filename) leaked its tail into the摘要 — `[x](a(b)c)`
    // came out `xc)`. The Rust indexer's `find_close_paren` is depth-balanced;
    // the flattener must agree or the underline sits on a phantom.
    expect(toPlainText("[x](a(b)c)")).toBe("x");
    expect(toPlainText("[nested](a(b(c)d)e)")).toBe("nested");
    expect(toPlainText("see [w](https://en.wikipedia.org/wiki/A_(b)) here")).toBe("see w here");
  });

  it("keeps an ESCAPED bracket inside the label (one link, not literal text)", () => {
    // L962/②: `[a\]b](u)` used to survive VERBATIM — the `\]` did not close
    // the label for the regex, so no match, so the raw markdown (target
    // included) landed in the row. The escape means the bracket is CONTENT:
    // the label is `a]b` and the whole construct still loses its target.
    expect(toPlainText("[a\\]b](u)")).toBe("a]b");
    expect(toPlainText("see [docs\\[beta\\]](https://x.y/z) now")).toBe("see docs[beta] now");
    // A backslash before an ordinary punctuation character keeps that
    // character and drops the backslash (no unstripped `\` reaches the row).
    expect(toPlainText("[a\\-b](u)")).toBe("a-b");
    expect(toPlainText("[a\\!b](u)")).toBe("a!b");
  });

  it("flattens an image to its alt text with a balanced target too", () => {
    expect(toPlainText("![alt](a(b)c.png)")).toBe("alt");
    // The badge idiom the docs actually use: an image wrapped in a link.
    expect(toPlainText("[![badge](https://img.shields.io/x.svg)](https://ci)")).toBe("badge");
    expect(toPlainText("![](https://x.y/z.png)")).toBe("");
  });

  it("leaves a lone `[` as prose (no phantom link, no swallowed line)", () => {
    // Half-typed links are common in the window BEFORE the target is closed;
    // the scan must not invent a link, nor eat text to the next `]`.
    expect(toPlainText("unclosed [ label")).toBe("unclosed [ label");
    expect(toPlainText("[t](u) trailing")).toBe("t trailing");
    expect(toPlainText("pre[link](https://x.y/z)post")).toBe("prelinkpost");
    // Reference-style labels are prose here (the excerpt path never resolves
    // them); only the INLINE form is flattened.
    // MUT:reference-labels-prose — teaching the flattener to resolve or drop
    // the `[r]` label turns the next two assertions RED (`t[r]` becomes `t`,
    // and the definition line stops surviving verbatim).
    expect(toPlainText("[t](u)[r]")).toBe("t[r]");
    expect(toPlainText("[a]: https://x.y/z")).toBe("[a]: https://x.y/z");
  });

  it("does not treat an ESCAPED bracket as a link opener", () => {
    // `\[` is a literal bracket in GFM, so this is NOT a link — the old regex
    // matched from index 1 and DELETED the visible `[-]` (`\[-](u)` → `\-`).
    // The scan keeps every visible character.
    //
    // The escape rules are the TS flattener's OWN (Rust's
    // `strip_inline_markers_into` has no `\` arm, so the two sides knowingly
    // diverge here — see the `buildBalanceMaps` note). A backslash suppresses
    // a STRUCTURAL character's role, and the resulting plain-text label gets a
    // full ASCII unescape — but PROSE outside any label is never unescaped.
    // MUT:label-only-unescape — unescaping the WHOLE string instead of just
    // the label turns the `\[x](u)` assertion below RED (done in R4).
    // Pin the prose half explicitly: `\[x](u)` must
    // stay `\[x](u)`, because GFM renders `\[` as a literal `[` so this is
    // not a link, and it must NOT be rewritten into `x`.
    expect(toPlainText("\\[x](u)")).toBe("\\[x](u)");
    // NOTE ON STAGE: this asserts the output of the WHOLE `toPlainText`
    // pipeline, and the value `\not emphasis\` is shaped by the LATER
    // `[*`~]` stripper (it removes the two `*`s, leaving the backslashes).
    // So this line pins the composition of two stages, not the link scan
    // alone — kept because it is the only guard that the emphasis stage still
    // runs AFTER the link stage and does not eat the backslashes.
    // MUT:escape-in-label — deleting the backslash suppression in
    // `buildBalanceMaps` turns the `[a\]b](u)` assertion above RED (done in R4).
    expect(toPlainText("\\*not emphasis\\*")).toBe("\\not emphasis\\");
    const escaped = toPlainText("\\[-](u)");
    expect(escaped).toContain("[-](u)");
    expect(toPlainText("see \\[not a link](u) here")).toContain("[not a link](u)");
    expect(toPlainText("see \\[not a link](u) here")).toContain("see");
  });

  it("caps link nesting, and the cap degrades to NOISE, not to data loss", () => {
    // L962/R2 S2-1. The cap exists for COST, not cosmetics: each nesting level
    // re-derives its own pair maps, so without the cap the cost is quadratic in
    // depth and the recursion eventually overflows the stack — one pasted line
    // could stall, then blank the whole row. No durations or depth thresholds
    // are recorded here on purpose: both vary with the engine, the stack limit
    // and the machine, and a number that cannot be kept true is worse than none.
    //
    // Shipped behaviour, pinned so the next round cannot quietly "fix" it into
    // a silent flatten and call it a bugfix: `depth > 8` returns the remainder
    // VERBATIM, so past the cap the markdown syntax — target included — is
    // visible noise. Depth is 0-based, so 9 levels still flatten correctly and
    // 10 is the first leaking depth.
    const nest = (levels: number): string => {
      let out = "x";
      for (let i = 0; i < levels; i += 1) out = `[${out}](u${i})`;
      return out;
    };

    // Correctly flattened at and below the boundary...
    for (const levels of [1, 2, 5, 8, 9]) {
      expect(toPlainText(nest(levels)), `nest(${levels}) must flatten`).toBe("x");
    }
    // ...and the KNOWN degradation one level past it. `nest(10)` is 61 chars,
    // so this is comfortably inside a real `PREVIEW_CHARS` window — the reason
    // the test says "noise" is honest rather than academic.
    expect(toPlainText(nest(10))).toBe("[x](u0)");
    expect(toPlainText(nest(10))).toContain("u0");

    // The invariant that makes the cap ACCEPTABLE: it only ever ADDS text the
    // page does not show. No input may lose visible characters, so the deepest
    // case still yields the label `x` and every level's bracket count is the
    // only thing that grows.
    // MUT:cap-degrades-to-noise — lowering or raising MAX_LINK_NESTING past
    // its boundary turns the `nest(9)`/`nest(10)` assertions above RED (CAP 1,
    // 3, 7 and 9 were each run in R4); the no-visible-loss half was checked
    // separately over depths 9..40 with zero characters lost.
    const deep = toPlainText(nest(30));
    expect(deep, "visible text survives the cap").toContain("x");
    expect(deep.length).toBeGreaterThan(0);

    // Unreachable-by-real-docs sanity: real nesting is two levels
    // (`[![alt](img)](href)`), which is far under the cap.
    expect(toPlainText("[![badge](i.svg)](href)")).toBe("badge");
  });

  it("stays LINEAR on an unterminated-`[` window at the PREVIEW_CHARS cap", () => {
    // L962/①: every unterminated `[` used to rescan to end-of-string, which
    // makes the cost QUADRATIC in the window length. That complexity is the
    // durable fact; the old form's actual durations are deliberately not
    // recorded here because they vary with the machine and the engine. The
    // cap matters because `PREVIEW_CHARS` (search.rs) bounds a shipped window,
    // and the hit list flattens once per row per keystroke (`debounce={0}`).
    //
    // The assertion is on the SHAPE (the large/small ratio), not a wall-clock
    // budget: an absolute bound is a machine-speed test that flakes on a
    // loaded box, while the ratio is what "linear" actually means. The two
    // inputs below differ by 8x, so quadratic arithmetic predicts ~64x and
    // linear arithmetic predicts ~8x; the threshold sits at 20x, which leaves
    // a linear pass ample margin while a regression to quadratic cannot fit.
    // These are ratios derived from the input sizes, not measured durations. comment-gate-ok: a disclaimer against misreading, with no behaviour to mutate; the threshold itself is pinned by the `toBeLessThan(20)` call right below.
    const adversarial = (chars: number): string => "[a".repeat(Math.ceil(chars / 2)).slice(0, chars);
    /** Median of `reps` runs — one noisy sample must not decide a red. */
    const median = (fn: (s: string) => unknown, input: string, reps: number): number => {
      const samples: number[] = [];
      for (let i = 0; i < reps; i += 1) {
        const start = performance.now();
        fn(input);
        samples.push(performance.now() - start);
      }
      samples.sort((a, b) => a - b);
      return samples[(samples.length - 1) >> 1]!;
    };
    const small = adversarial(4096);
    const large = adversarial(32768);
    median(toPlainText, small, 15); // warm the JIT before measuring
    median(toPlainText, large, 5);
    const smallMs = Math.max(median(toPlainText, small, 15), 0.05);
    const largeMs = median(toPlainText, large, 5);
    expect(largeMs / smallMs).toBeLessThan(20);

    // The live cap: one `PREVIEW_CHARS`-sized window must stay imperceptible
    // even with a generous ceiling.
    const capped = adversarial(8192);
    expect(median(toPlainText, capped, 5)).toBeLessThan(20);
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

  it("counts GRAPHEME clusters in the budget (a flag is one character)", () => {
    // The R2 review's S1: a code-point budget cut INSIDE clusters — the
    // family emoji printed a broken torso, a flag reassembled into a
    // DIFFERENT country, a combining accent dangled. The budget is
    // user-perceived characters, and the invariant is CLUSTER INTEGRITY:
    // every cluster in the kept context is a complete grapheme (a whole
    // family, a whole flag) — never a bare ZWJ, a lone face or a single
    // regional indicator. (ZWJ DOES appear inside an intact family cluster;
    // the earlier substring regex caught that and was wrong.)
    const FAMILY = "👨‍👩‍👧‍👦";
    const lines = buildExcerptLines(`${FAMILY}${FAMILY}${FAMILY} 测试 后面还有一些内容`, ["测试"]);
    // Only the BEFORE side: everything left of the match segment (an
    // earlier draft joined both sides and flagged the AFTER context's
    // perfectly legitimate hanzi clusters as "broken").
    const matchAt = lines[0].segments.findIndex((x) => x.match);
    const before = lines[0].segments.slice(0, matchAt).map((x) => x.text).join("");
    const split = (v: string): string[] => {
      const Seg = (globalThis as { Intl?: { Segmenter?: new (l: string, o: { granularity: string }) => { segment(v: string): Iterable<{ segment: string }> } } }).Intl?.Segmenter;
      return Seg
        ? [...new Seg("und", { granularity: "grapheme" }).segment(v)].map((part) => part.segment)
        : Array.from(v);
    };
    const clusters = split(before);
    for (const cluster of clusters) {
      expect(cluster === " " || cluster === FAMILY).toBe(true);
    }
    expect(clusters.length).toBeLessThanOrEqual(6);

    const flags = buildExcerptLines("🇨🇳🇨🇳🇨🇳🇨🇳 测试 尾部还有", ["测试"]);
    const flagMatchAt = flags[0].segments.findIndex((x) => x.match);
    const flagBefore = flags[0].segments.slice(0, flagMatchAt).map((x) => x.text).join("");
    for (const cluster of split(flagBefore)) {
      expect(cluster === " " || cluster === "🇨🇳").toBe(true);
    }
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