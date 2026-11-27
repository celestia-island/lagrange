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

export interface ExcerptSegment {
  text: string;
  match: boolean;
}

/** One match's neighbourhood: `…before[MARK]after…` as segments. */
export interface ExcerptLine {
  segments: ExcerptSegment[];
}

/** Keep the last `limit` characters of `text`, preferring a boundary. */
function tailWithin(text: string, limit: number): string {
  const chars = Array.from(text);
  if (chars.length <= limit) return text;
  const from = chars.length - limit;
  for (let i = from; i < chars.length - 1; i += 1) {
    if (BOUNDARY.test(chars[i] ?? "")) return chars.slice(i + 1).join("");
  }
  return chars.slice(from).join("");
}

/** Keep the first `limit` characters of `text`, preferring a boundary. */
function headWithin(text: string, limit: number): string {
  const chars = Array.from(text);
  if (chars.length <= limit) return text;
  for (let i = limit; i > 1; i -= 1) {
    if (BOUNDARY.test(chars[i - 1] ?? "")) return chars.slice(0, i - 1).join("");
  }
  return chars.slice(0, limit).join("");
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
  // Links / images → their label only.
  text = text.replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1");
  text = text.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
  // Emphasis / inline code / strikethrough markers.
  text = text.replace(/(\*\*|__|\*|_|`+|~~)/g, "");
  // HTML tags (the chunk source can carry them).
  text = text.replace(/<[^>]*>/g, " ");

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
    const head = headWithin(text, CONTEXT_CHARS * 4);
    return [
      {
        segments: [
          { text: head, match: false },
          ...(head.length < text.length ? [{ text: ELLIPSIS, match: false }] : []),
        ],
      },
    ];
  }

  return matches.slice(0, MAX_EXCERPTS).map(({ at, len }) => {
    const beforeRaw = text.slice(Math.max(0, at - CONTEXT_CHARS * 4), at);
    const afterRaw = text.slice(at + len, at + len + CONTEXT_CHARS * 4);
    const before = tailWithin(beforeRaw, CONTEXT_CHARS);
    const after = headWithin(afterRaw, CONTEXT_CHARS);
    const elidedBefore = at > before.length;
    const elidedAfter = text.length - (at + len) > after.length;
    const segments: ExcerptSegment[] = [];
    if (elidedBefore) segments.push({ text: ELLIPSIS, match: false });
    if (before.length > 0) segments.push({ text: before, match: false });
    segments.push({ text: text.slice(at, at + len), match: true });
    if (after.length > 0) segments.push({ text: after, match: false });
    if (elidedAfter) segments.push({ text: ELLIPSIS, match: false });
    return { segments };
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
