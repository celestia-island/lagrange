// Compact, uniform search-hit excerpts.
//
// The problem (2026-11-27 user direction, with two phone screenshots):
// rendering each hit's markdown window VERBATIM made result rows wildly
// different heights — a table chunk rendered a five-row table (267px) and
// the tall card pushed its own match below the clamp, so the card showed
// NO underline at all even though the match was there. A one-line
// paragraph rendered 75px beside it. The user's own prescription: "自动
// 分析正文和将搜索到的文本前后的文本都换成省略号、搜索到的文本前后各自
// 只保留 6 个字，这样就能同时做到高度限制的同时又能尽可能多展示搜索到的
// 结果". Keep six characters of context each side of the match, elide the
// rest.
//
// That rule is what makes the list uniform AND keeps the match visible:
// every row is one line per match, of bounded width, so rows line up and
// the underlined match is always inside the visible line.
//
// Why flatten to text instead of squeezing the markdown in place: a table
// is STILL five rows after its cells are shortened, so the "hard to
// compress" structures — tables, lists, code blocks — must lose their
// block shape to become uniform. Flattening to text is exactly the
// "自动分析正文" step: block syntax disappears, reading order survives.

/** Characters of context kept on each side of a match (user budget). */
export const CONTEXT_CHARS = 6;

/** Cap on how many matches one excerpt shows — each is a line, and a row
 *  with ten lines is not uniform either. */
const MAX_EXCERPTS = 2;

const ELLIPSIS = "…";

/** Boundary characters preferred when trimming a ragged edge: whitespace
 *  plus the CJK/fullwidth punctuation ranges a Chinese excerpt uses. */
const BOUNDARY = /[\s\u3000-\u303f\uff00-\uffef\u2000-\u206f]/;

/** Split `text` into USER-PERCEIVED characters, not code points: a ZWJ
 *  emoji, a flag (two regional indicators) or a combining accent is ONE
 *  character to the reader, and a budget that counts code points mangles
 *  all three (a cut inside 👨‍👩‍👧‍👦 prints a broken torso; a cut inside a
 *  flag prints a DIFFERENT country). `Intl.Segmenter` is the platform's
 *  cluster splitter (every evergreen browser, Node ≥16); code points are
 *  the honest fallback where it is missing — same as the pre-Segmenter
 *  behaviour, never worse. */
function segments(text: string): string[] {
  const Seg = (globalThis as { Intl?: { Segmenter?: new (...args: unknown[]) => { segment(s: string): Iterable<{ segment: string }> } } })
    .Intl?.Segmenter;
  if (Seg) {
    const splitter = new Seg("und", { granularity: "grapheme" }) as { segment(s: string): Iterable<{ segment: string }> };
    return [...splitter.segment(text)].map((part) => part.segment);
  }
  return Array.from(text);
}

export interface ExcerptSegment {
  text: string;
  match: boolean;
}

/** One match's neighbourhood: `…before[MARK]after…` as segments. */
export interface ExcerptLine {
  segments: ExcerptSegment[];
}

/** Keep the last `limit` CLUSTERS of `segs`, preferring to start after a
 *  boundary cluster (never mid-word). Always keeps at least one. */
function tailClusters(segs: string[], limit: number): string[] {
  if (segs.length <= limit) return segs;
  const window = segs.slice(-limit);
  for (let i = 0; i < window.length - 1; i += 1) {
    if (BOUNDARY.test(window[i] ?? "")) return window.slice(i + 1);
  }
  return window;
}

/** Keep the first `limit` CLUSTERS of `segs`, preferring to end before a
 *  boundary cluster. Always keeps at least one. */
function headClusters(segs: string[], limit: number): string[] {
  if (segs.length <= limit) return segs;
  const window = segs.slice(0, limit);
  for (let i = window.length - 1; i > 0; i -= 1) {
    if (BOUNDARY.test(window[i] ?? "")) return window.slice(0, i);
  }
  return window;
}

/**
 * Flatten markdown source to readable single-space text.
 *
 * This is the "analyze the body" step, not a renderer: the excerpt is a
 * one-line context, so block syntax (table pipes, list bullets, heading
 * hashes, fences, emphasis markers) must go while the WORDS stay in
 * reading order. Inline links keep their label and lose their target —
 * a URL would eat the six-character budget.
 */
export function toPlainText(markdown: string): string {
  let text = markdown;

  // Fenced code: keep the code lines, drop the fences.
  text = text.replace(/^ {0,3}(?:```|~~~)[^\n]*$/gm, " ");
  // Heading / blockquote / list markers at line start.
  text = text.replace(/^ {0,3}#{1,6}[ \t]+/gm, " ");
  text = text.replace(/^ {0,3}>[ \t]?/gm, " ");
  text = text.replace(/^ {0,3}(?:[-*+]|\d+[.)])[ \t]+/gm, " ");
  // Table rows: drop the separator line, then turn cell pipes into spaces.
  text = text.replace(/^ {0,3}\|?[ \t]*:?-{3,}:?[ \t]*(?:\|[ \t]*:?-{3,}:?[ \t]*)*\|?[ \t]*$/gm, " ");
  text = text.replace(/[ \t]*\|[ \t]*/g, " ");
  // Links / images → their label only. Labels and targets are bounded to
  // ONE LINE (inline links do not span lines in GFM): an unbounded `[^)]*`
  // made every unterminated `[` rescan to end-of-string, which is quadratic
  // (measured 5.4 s at 128 KB — the amplifier for the window-size fix above,
  // R5/S3).
  text = text.replace(/!\[([^\]\n]*)\]\(([^)\n]*)\)/g, "$1");
  text = text.replace(/\[([^\]\n]*)\]\(([^)\n]*)\)/g, "$1");
  // Emphasis / inline code / strikethrough markers. PAIRED markers go with
  // their content kept; lone `*` / backtick / `~` are markers too. A lone
  // `_` is NOT: snake_case identifiers are content (and GFM agrees —
  // intraword underscores never emphasize), so `use_modal_state` must
  // survive flattening searchable.
  text = text.replace(/(\*\*|__|~~|`+)(\S[^]*?\S|\S)\1/g, "$2");
  text = text.replace(/(^|[\s(])_(\S[^_\n]*?\S)_(?=$|[\s.,;:!?)\]）」])/g, "$1$2");
  text = text.replace(/[*`~]/g, "");
  // HTML tags (the chunk source can carry them). The letter requirement is
  // load-bearing: `a < b > c` in prose is comparison operators, not a tag,
  // and the greedy `[^>]*` form deleted the word between them.
  text = text.replace(/<\/?[a-zA-Z][^<>]*>/g, " ");

  // Collapse whitespace LAST so the earlier regexes could anchor on lines.
  return text.replace(/\s+/g, " ").trim();
}

/** Locate every query-token occurrence in `text`, merging overlaps so one
 *  visual span is never emitted twice. */
function findMatches(text: string, tokens: string[]): Array<{ at: number; len: number }> {
  const usable = tokens.filter((t) => t.length > 0);
  if (usable.length === 0) return [];
  const lower = text.toLowerCase();
  const raw: Array<{ at: number; len: number }> = [];
  for (const token of usable) {
    const needle = token.toLowerCase();
    let from = 0;
    for (;;) {
      const at = lower.indexOf(needle, from);
      if (at < 0) break;
      raw.push({ at, len: needle.length });
      from = at + needle.length;
    }
  }
  raw.sort((a, b) => a.at - b.at || b.len - a.len);
  const merged: Array<{ at: number; len: number }> = [];
  for (const span of raw) {
    const last = merged[merged.length - 1];
    if (last && span.at < last.at + last.len) {
      last.len = Math.max(last.len, span.at + span.len - last.at);
    } else {
      merged.push({ ...span });
    }
  }
  return merged;
}

/**
 * Build the excerpt lines for one hit.
 *
 * `markdown` is the chunk's markdown window (as shipped in the index);
 * `rawQuery` is the user's query. Returns at most `MAX_EXCERPTS` lines,
 * each `…context[match]context…`, with the elision marks emitted ONLY on
 * a side that really dropped text.
 */
export function buildExcerptLines(markdown: string, tokens: string[]): ExcerptLine[] {
  const text = toPlainText(markdown);
  if (text.length === 0) return [];
  const matches = findMatches(text, tokens);
  if (matches.length === 0) {
    // No literal occurrence (the index matched a token the flattened text
    // spells differently, e.g. split by markup): show the opening context
    // rather than an empty row.
    const clusters = segments(text);
    const kept = headClusters(clusters, CONTEXT_CHARS * 2);
    return [
      {
        segments: [
          { text: kept.join(""), match: false },
          ...(clusters.length > kept.length ? [{ text: ELLIPSIS, match: false }] : []),
        ],
      },
    ];
  }

  // ONE segmentation of the whole flattened text; every context choice
  // below slices THIS array, so no cut can ever land inside a cluster. A
  // raw `text.slice(at - budget, at)` pre-cut was tried first and the
  // grapheme test caught it: slicing mid-emoji feeds the Segmenter a
  // broken string, and no downstream cluster-aware budgeting can repair
  // an already-shattered cluster.
  const clusters = segments(text);
  // UTF-16 start offset of each cluster: findMatches speaks UTF-16
  // indices (indexOf), so map match offsets onto cluster indices.
  const starts: number[] = [];
  let walked = 0;
  for (const seg of clusters) {
    starts.push(walked);
    walked += seg.length;
  }
  const segAt = (offset: number): number => {
    let lo = 0;
    let hi = starts.length - 1;
    let ans = 0;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (starts[mid]! <= offset) {
        ans = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    return ans;
  };

  return matches.slice(0, MAX_EXCERPTS).map(({ at, len }) => {
    const first = segAt(at);
    const last = segAt(at + len - 1);
    const beforeAll = clusters.slice(0, first);
    const afterAll = clusters.slice(last + 1);
    const keptBefore = tailClusters(beforeAll, CONTEXT_CHARS);
    const keptAfter = headClusters(afterAll, CONTEXT_CHARS);
    const before = keptBefore.join("");
    const after = keptAfter.join("");
    const elidedBefore = beforeAll.length > keptBefore.length;
    const elidedAfter = afterAll.length > keptAfter.length;
    const lineSegments: ExcerptSegment[] = [];
    if (elidedBefore) lineSegments.push({ text: ELLIPSIS, match: false });
    if (before.length > 0) lineSegments.push({ text: before, match: false });
    lineSegments.push({ text: text.slice(at, at + len), match: true });
    if (after.length > 0) lineSegments.push({ text: after, match: false });
    if (elidedAfter) lineSegments.push({ text: ELLIPSIS, match: false });
    return { segments: lineSegments };
  });
}

/** Block constructs that cannot be squeezed into a one-line excerpt: after
 *  their cells/items are shortened a table is STILL five rows, a list is
 *  still N bullets. These are what the user called "列表搜索结果或者不太好
 * 压缩的部分", and they take the flattened path. */
const BLOCK_STRUCTURE =
  /^ {0,3}(?:\|.*\|\s*$|[-*+][ \t]+|\d+[.)][ \t]+|```|~~~)/m;

/** True when the window's markdown carries block structure that would not
 *  compress — the "自动分析正文" verdict. A plain prose window keeps its
 *  markdown rendering (the reader sees real emphasis and links). */
export function needsCollapse(markdown: string): boolean {
  return BLOCK_STRUCTURE.test(markdown);
}
