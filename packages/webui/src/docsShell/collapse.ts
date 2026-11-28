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

/** Deepest link-inside-link nesting the flattener unrolls.
 *
 *  WHY A CAP AT ALL: the recursion deepens once per nesting level and each
 *  level re-derives its own pair maps, so without the cap the cost is
 *  QUADRATIC in the nesting depth of a link-in-link chain — that is a property
 *  of the shape of the code, not a measurement — and the recursion eventually
 *  overflows the stack, so a single pasted line could stall and then throw the
 *  whole excerpt away. The cap is a constant bound, which is what keeps the
 *  whole projection linear in the window for arbitrarily hostile input.
 *
 *  No depth or duration figures are recorded here on purpose. The overflow
 *  point and the wall-clock costs both depend on the engine, the stack limit
 *  and the machine, so any specific number would be a claim this file cannot
 *  keep true. What the cap needs to guarantee is only that it is BOUNDED and
 *  comfortably below any plausible limit.
 *
 *  WHAT HAPPENS PAST IT (asserted in the tests, do NOT "fix" this into a
 *  silent flatten): the too-deep remainder is returned VERBATIM, so its
 *  markdown syntax — including its `(href)` target — survives into the
 *  excerpt as visible noise. That is a known, bounded degradation:
 *
 *  - It is NOISE, never data loss: every character still on the page is still
 *    emitted, so the `…context[match]context…` contract and the underline
 *    invariant hold. (It shows text the page does NOT show — the target — which
 *    is why it is noise and why `escapeHtml` still covers it: no injection.)
 *  - Depth 10 already leaks, so the cap and the first leak coincide: with
 *    `MAX_LINK_NESTING = 8` the deepest CORRECT flatten is 9 levels
 *    (`depth` is 0-based and the guard is `depth > MAX_LINK_NESTING`), and
 *    depth ≥ 10 leaks `[x](u)`.
 *  - Reachable in principle: each nesting level costs only a handful of
 *    characters, so a `PREVIEW_CHARS` window can hold far more levels than the
 *    cap allows. Real docs nest two (`[![alt](img)](href)`); a hand-written
 *    ten-deep link-in-link chain is a corpus nobody writes, and if one appears
 *    the excerpt is noisy for that row only — not wrong, not unsafe, not slow. */
const MAX_LINK_NESTING = 8;

/** ASCII punctuation — the only characters a backslash can escape in GFM —
 *  as a replacement pattern: an escaped bracket becomes the bracket it
 *  escaped, which is what the reader was always meant to see. Applied to a
 *  flattened LABEL only; see the two escape rules on `buildBalanceMaps`. */
const BACKSLASH_ESCAPE = /\\([\u0021-\u002f\u003a-\u0040\u005b-\u0060\u007b-\u007e])/g;

/** Balanced-pair maps for ONE string: `brackets[i]` is the index of the `]`
 *  that closes the `[` at `i` (and `parens[i]` likewise for `(`), or -1 when
 *  the pair never closes on this line.
 *
 *  Built ONCE per string in a single left-to-right pass, because that single
 *  pass is the whole point: the two regexes this replaced rescanned to
 *  end-of-string for EVERY unterminated `[`, which is QUADRATIC in the window
 *  length. That is why the old form got worse super-linearly as the window
 *  grew (no durations are recorded here — they vary with the machine and the
 *  engine; the complexity is the durable fact). One pass is LINEAR regardless
 *  of how many brackets never close.
 *
 *  Depth semantics mirror the Rust indexer (`src/search.rs`:
 *  `find_close_bracket` / `find_close_paren`): the close is the FIRST one at
 *  depth zero, not the first close CHARACTER — that is what makes
 *  `[x](a(b)c)` keep href `a(b)c`. The agreement claimed here is BALANCE ONLY
 *  (depth counting); it does not cover newline handling (see the stack-reset
 *  note below, where the two do differ) and it does not cover the ESCAPE
 *  semantics, which differ too and are called out rather than glossed.
 *
 *  Escape semantics are the TS flattener's OWN rules, closer to how GFM
 *  RENDERS than to how the indexer SCANS. There is no Rust counterpart to
 *  point at: `strip_inline_markers_into` (`src/search.rs:848`) has five match
 *  arms (`` ` ``, `*`/`_`, `![`, `[`, `<`) and NO `\` arm, and the
 *  `find_close_bracket` / `find_close_paren` it calls do zero escape
 *  handling. Two rules, both stated as what this file does:
 *
 *  (a) STRUCTURAL characters (`\[`, `\]`, `\(`, `\)`) — a backslash suppresses
 *      the character's structural role, so `[a\]b](u)` is one link whose label
 *      is `a]b` rather than a link ending at the escaped bracket.
 *
 *      ⚠️ THIS KNOWINGLY DIVERGES FROM THE RUST INDEXER. Compiled and run
 *      against the real `find_close_bracket`: the same input
 *      `[a\]b](u)` yields `close = 3`, i.e. label `"a\"` — Rust treats the
 *      escaped `\]` as the closing bracket and TS does not. The two are
 *      OPPOSITE on this input, by design. The divergence is accepted because
 *      it only affects which characters an excerpt SHOWS; the underline
 *      contract is carried by token positions the index ships, not by this
 *      projection, and `escapeHtml` covers the text either way. Do not
 *      "restore consistency" by copying the indexer's escape-free walk: that
 *      would print `a\` and lose the `b` the page shows.
 *  (b) INSIDE a flattened label — a full ASCII-punctuation unescape, because
 *      the label is now plain text and must not carry `\]` through to the row.
 *      It never reaches outside a label: prose keeps whatever backslashes it
 *      shipped with, so `\[x](u)` stays `\[x](u)` (GFM renders `\[` as a
 *      literal `[`, so this prose is NOT a link and must not be expanded) and
 *      `\*not em\*` keeps its backslashes untouched.
 *
 *  A newline RESETS both stacks. That is THIS flattener's own choice, and it
 *  is stated as such rather than justified by a standard: it does not match
 *  GFM, which does allow an inline link to span lines, and it does not match
 *  the indexer's `find_close_bracket`, which has no newline handling at all.
 *  It is kept because the excerpt is a one-line projection by construction, so
 *  a stray `[` must not gobble the rest of the chunk. */
function buildBalanceMaps(source: string): { brackets: Int32Array; parens: Int32Array } {
  const length = source.length;
  const brackets = new Int32Array(length).fill(-1);
  const parens = new Int32Array(length).fill(-1);
  const bracketStack: number[] = [];
  const parenStack: number[] = [];
  for (let i = 0; i < length; i += 1) {
    const ch = source.charCodeAt(i);
    if (ch === 10 /* \n */) {
      bracketStack.length = 0;
      parenStack.length = 0;
      continue;
    }
    if (ch === 92 /* \ */) {
      // The escaped character is content: it may not open or close a pair.
      // A backslash before the newline escapes nothing.
      if (i + 1 < length && source.charCodeAt(i + 1) !== 10) i += 1;
      continue;
    }
    if (ch === 91 /* [ */) {
      bracketStack.push(i);
    } else if (ch === 93 /* ] */) {
      const open = bracketStack.pop();
      if (open !== undefined) brackets[open] = i;
    } else if (ch === 40 /* ( */) {
      parenStack.push(i);
    } else if (ch === 41 /* ) */) {
      const open = parenStack.pop();
      if (open !== undefined) parens[open] = i;
    }
  }
  return { brackets, parens };
}

/** `[text](target)` / `![alt](target)` → `text` / `alt`.
 *
 *  A scan, not a replacement: balanced pairs are not a regular language. The
 *  label is flattened RECURSIVELY the way the indexer does it, so
 *  `[![alt](i.png)](u)` yields `alt` and `[a[b]c](u)` yields `a[b]c` instead
 *  of leaving half a construct for the emphasis strippers to maul. Every
 *  construct the scan consumes advances `i` past its target, so the caller's
 *  text is walked exactly once. */
function flattenLinks(source: string, depth = 0): string {
  if (depth > MAX_LINK_NESTING) return source;
  const { brackets, parens } = buildBalanceMaps(source);
  const parts: string[] = [];
  const length = source.length;
  let literalStart = 0;
  let i = 0;
  while (i < length) {
    const ch = source.charCodeAt(i);
    if (ch === 92 /* \ */) {
      // Escaped character: literal content, never a link opener.
      i += 2;
      continue;
    }
    const image = ch === 33 /* ! */ && i + 1 < length && source.charCodeAt(i + 1) === 91 /* [ */;
    if (image || ch === 91 /* [ */) {
      const open = image ? i + 1 : i;
      const close = brackets[open];
      if (close >= 0 && source.charCodeAt(close + 1) === 40 /* ( */) {
        const targetClose = parens[close + 1];
        if (targetClose >= 0) {
          parts.push(source.slice(literalStart, i));
          const label = source.slice(open + 1, close);
          // Recursing is only worth it when the label can hold a construct of
          // its own. The `[` probe is a sufficient short-circuit, not a guess:
          // every construct this scan can consume starts with `[` — a link
          // does, and a nested IMAGE does too, because its `!` is the
          // character immediately BEFORE the `[` (`[![alt](i.png)](u)` puts
          // `![` inside the label, so the `[` is present). A label without a
          // `[` can therefore only be flat text, and skipping the walk for it
          // is what keeps the common case allocation-free.
          //
          // The unescape applies to the LABEL ONLY — it is the one place this
          // scan rewrites characters, and text outside every link keeps
          // whatever backslashes it shipped with. An earlier draft unescaped
          // the whole string instead; differential fuzzing against that draft
          // showed it silently rewrote prose this fix is not about, which is
          // why the unescape is scoped to this line.
          const nested = label.includes("[") ? flattenLinks(label, depth + 1) : label;
          parts.push(label.includes("\\") ? nested.replace(BACKSLASH_ESCAPE, "$1") : nested);
          i = targetClose + 1;
          literalStart = i;
          continue;
        }
      }
    }
    i += 1;
  }
  parts.push(source.slice(literalStart));
  return parts.join("");
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
  // Links / images → their label only. A linear scan (not a regex) because
  // balanced pairs are not a regular language: `[x](a(b)c)` has href
  // `a(b)c` in GFM, and the previous `[^)\n]*` target stopped at the FIRST
  // `)`, leaking `c)` into the excerpt. Escaped brackets (`[a\]b](u)`) come
  // out as `a]b`.
  //
  // The BALANCED-PAIR half matches the indexer (`find_close_bracket` /
  // `find_close_paren` count depth the same way) — BALANCE ONLY, not newline
  // handling, which differs. The ESCAPE half does NOT match, and pretending
  // otherwise would be a false claim: Rust's
  // `strip_inline_markers_into` has no `\` arm, so `[a\]b](u)` gives the index
  // the label `a\` while this flattener gives `a]b`. That is a KNOWN,
  // DELIBERATE divergence — see the long note on `buildBalanceMaps` for why it
  // is safe (display-only; the underline contract rides on token positions,
  // not on this projection) and why "restoring consistency" is the wrong fix.
  text = flattenLinks(text);
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
