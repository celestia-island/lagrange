// Match-centred snippet windows for the docs search hits (L942 follow-up).
//
// A hit used to show the chunk's first 160 characters — raw markdown, table
// pipes and all, and any match deeper in the section was invisible. The view
// the user asked for is a MAGNIFIER: cut a window of the chunk's real text
// around the first match, render it AS markdown (pipe rows become tables,
// markers become structure), underline every query occurrence in place, and
// fade the window's top/bottom edge only where the section text continues
// beyond it (no fade at the section's true start/end).

import { marked } from "marked";
import DOMPurify from "dompurify";

import { tokenizeQuery } from "./search";

/** The visible window's radius around the first match, in characters. The
 *  window itself is line-snapped afterwards so markdown blocks (tables,
 *  lists) survive the cut. */
const WINDOW_RADIUS = 160;

/** A token occurrence inside `text` at/after `from` (case-insensitive for
 *  ASCII tokens; CJK bigrams match exactly — mirrors the indexer's own
 *  tokenization contract, see search.ts). */
function findTokenAt(text: string, token: string, from: number): number {
  if (/^[a-z0-9]+$/.test(token)) {
    const lower = text.toLowerCase();
    return lower.indexOf(token, from);
  }
  return text.indexOf(token, from);
}

/** Earliest occurrence of ANY query token in `text` (-1 when none). */
function firstMatch(text: string, tokens: string[]): { index: number; length: number } | null {
  let best: { index: number; length: number } | null = null;
  for (const token of tokens) {
    if (token.length < 1) continue;
    const at = findTokenAt(text, token, 0);
    if (at >= 0 && (best === null || at < best.index)) best = { index: at, length: token.length };
  }
  return best;
}

/** Cut a line-snapped window around `match` and report whether the section
 *  continues beyond either cut. */
export function sliceWindow(
  text: string,
  match: { index: number; length: number },
): { window: string; hasAbove: boolean; hasBelow: boolean } {
  let start = Math.max(0, match.index - WINDOW_RADIUS);
  let end = Math.min(text.length, match.index + match.length + WINDOW_RADIUS);
  // Line-snap outward (never inward): a cut that opens mid-table-row renders
  // broken markdown, so the window eats to the nearest line boundaries.
  if (start > 0) {
    const nl = text.lastIndexOf("\n", start);
    start = nl >= 0 ? nl + 1 : 0;
  }
  if (end < text.length) {
    const nl = text.indexOf("\n", end);
    end = nl >= 0 ? nl + 1 : text.length;
  }
  return {
    window: text.slice(start, end),
    hasAbove: start > 0,
    hasBelow: end < text.length,
  };
}

/** Wrap every query-token occurrence in a `<mark>` — used by the DOM walk
 *  over the RENDERED snippet, so matches light up inside the formatted
 *  output exactly where they sit in the source text. */
function markMatches(root: ParentNode, tokens: string[]): void {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const targets: Text[] = [];
  while (walker.nextNode()) targets.push(walker.currentNode as Text);
  for (const node of targets) {
    let text = node.data;
    let lower = text.toLowerCase();
    let touched = false;
    // Repeated single-pass scans: marking a match splits the node, so the
    // loop restarts over the remaining (unmarked) segments via the queue.
    const spans: Array<{ text: string }> = [];
    let rest = text;
    let cursor = 0;
    let guard = 0;
    scan: while (guard++ < 200) {
      let hit: { at: number; len: number } | null = null;
      for (const token of tokens) {
        if (token.length < 1) continue;
        const at = lower.indexOf(token);
        if (at >= 0 && (hit === null || at < hit.at)) hit = { at, len: token.length };
      }
      if (!hit) break;
      spans.push({ text: rest.slice(0, hit.at) });
      spans.push({ text: rest.slice(hit.at, hit.at + hit.len) });
      rest = rest.slice(hit.at + hit.len);
      lower = rest.toLowerCase();
      touched = true;
      cursor += hit.at + hit.len;
      continue scan;
    }
    void cursor;
    if (!touched) continue;
    spans.push({ text: rest });
    const parent = node.parentNode;
    if (!parent) continue;
    const frag = document.createDocumentFragment();
    for (const [i, span] of spans.entries()) {
      if (span.text.length === 0) continue;
      // Odd indexes are the matches (between the plain segments).
      if (i % 2 === 1) {
        const mark = document.createElement("mark");
        mark.className = "lagrange-docs-search-mark";
        mark.textContent = span.text;
        frag.appendChild(mark);
      } else {
        frag.appendChild(document.createTextNode(span.text));
      }
    }
    parent.replaceChild(frag, node);
    void text;
  }
}

export interface SnippetView {
  /** Sanitized HTML for the window, matches wrapped in `<mark>`. */
  html: string;
  /** The section text continues above the window's first line. */
  hasAbove: boolean;
  /** The section text continues below the window's last line. */
  hasBelow: boolean;
}

/** Build the match-centred, markdown-rendered snippet view for a hit. */
export function buildSnippetView(text: string, rawQuery: string): SnippetView {
  const tokens = tokenizeQuery(rawQuery).filter((t) => t.length >= 1);
  const match = tokens.length > 0 ? firstMatch(text, tokens) : null;
  const windowText = match
    ? sliceWindow(text, match)
    : { window: text.slice(0, WINDOW_RADIUS * 2), hasAbove: text.length > WINDOW_RADIUS * 2, hasBelow: false };

  // marked: the window is plain text kept in markdown shape (pipe rows,
  // list markers) — rendering it turns tables into tables. marked v18
  // defaults to synchronous string output.
  const raw = marked.parse(windowText.window, { async: false }) as string;
  const clean = DOMPurify.sanitize(raw, { USE_PROFILES: { html: true } });

  // Mark over the SANITIZED tree: the sanitizer would strip a raw <mark>
  // injected before it? (it wouldn't — mark is allowed — but marking after
  // sanitize keeps the regex/walk working on final HTML.)
  const holder = document.createElement("div");
  holder.innerHTML = clean;
  if (tokens.length > 0) markMatches(holder, tokens);

  return { html: holder.innerHTML, hasAbove: windowText.hasAbove, hasBelow: windowText.hasBelow };
}
