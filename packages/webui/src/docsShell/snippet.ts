// Match-centred snippet windows for the docs search hits.
//
// A hit used to show the chunk's first 160 characters of FLATTENED text —
// markdown table pipes and all, so the card read as a raw markdown dump, and
// any match deeper in the section was invisible. The view the user asked for
// is a MAGNIFIER: cut a line-snapped window of the chunk's MARKDOWN SOURCE
// around the first match, render it as markdown (pipe rows become tables,
// emphasis survives), underline every query occurrence in place, and fade the
// window's top/bottom edge only where the section text continues beyond it
// (no fade at the section's true start/end).

import { marked } from "marked";
import DOMPurify from "dompurify";

import { tokenizeQuery } from "./search";

/** The visible window's radius around the first match, in characters. The
 *  window itself is line-snapped afterwards so markdown blocks (tables,
 *  lists) survive the cut. */
const WINDOW_RADIUS = 160;

/** Sanitize against the CALL-TIME window. The dompurify default export is
 *  bound to whatever window existed at import time; in a test/SSR-adjacent
 *  graph that can be none, leaving a bare factory without `sanitize`.
 *  Calling the factory with the live window is the documented SSR path and
 *  is idempotent for an already-bound instance. */
type Purifier = { sanitize: (dirty: string, config?: Record<string, unknown>) => string };

function purify(html: string): string {
  const candidate = DOMPurify as unknown as Partial<Purifier> & ((w: unknown) => Purifier);
  const instance: Purifier =
    typeof candidate.sanitize === "function" ? (candidate as Purifier) : candidate(globalThis);
  return instance.sanitize(html, { USE_PROFILES: { html: true } });
}

/** A token occurrence inside `text` at/after `from` (case-insensitive for
 *  ASCII tokens; CJK bigrams match exactly — mirrors the indexer's own
 *  tokenization contract, see search.ts). */
function findTokenAt(text: string, token: string, from: number): number {
  if (/^[a-z0-9]+$/.test(token)) {
    return text.toLowerCase().indexOf(token, from);
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

/** Wrap every query-token occurrence in a `<mark>`. Walks the RENDERED tree's
 *  text nodes, so a match lights up inside the formatted output exactly where
 *  it sits in the source (never inside markup). */
function markMatches(root: ParentNode, tokens: string[]): void {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const targets: Text[] = [];
  while (walker.nextNode()) targets.push(walker.currentNode as Text);
  for (const node of targets) {
    let rest = node.data;
    // Escaping the loop is not a correctness path: a token always consumes
    // at least one character, so the remaining text strictly shrinks.
    const segments: Array<{ text: string; mark: boolean }> = [];
    let touched = false;
    for (;;) {
      let lower = rest.toLowerCase();
      let hit: { at: number; len: number } | null = null;
      for (const token of tokens) {
        if (token.length < 1) continue;
        const at = lower.indexOf(token);
        if (at >= 0 && (hit === null || at < hit.at)) hit = { at, len: token.length };
      }
      if (!hit) break;
      if (hit.at > 0) segments.push({ text: rest.slice(0, hit.at), mark: false });
      segments.push({ text: rest.slice(hit.at, hit.at + hit.len), mark: true });
      rest = rest.slice(hit.at + hit.len);
      touched = true;
    }
    if (!touched) continue;
    segments.push({ text: rest, mark: false });
    const parent = node.parentNode;
    if (!parent) continue;
    const frag = document.createDocumentFragment();
    for (const segment of segments) {
      if (segment.text.length === 0) continue;
      if (segment.mark) {
        const mark = document.createElement("mark");
        mark.className = "lagrange-docs-search-mark";
        mark.textContent = segment.text;
        frag.appendChild(mark);
      } else {
        frag.appendChild(document.createTextNode(segment.text));
      }
    }
    parent.replaceChild(frag, node);
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
    : {
        window: text.slice(0, WINDOW_RADIUS * 2),
        hasAbove: false,
        hasBelow: text.length > WINDOW_RADIUS * 2,
      };

  // The window is the chunk's MARKDOWN SOURCE: rendering it turns pipe rows
  // into tables and keeps emphasis. marked v18 emits a synchronous string.
  const raw = marked.parse(windowText.window, { async: false }) as string;
  const holder = document.createElement("div");
  holder.innerHTML = purify(raw);

  // A hit is an EXCERPT, not a page: a remote image would fire a request per
  // hit, and a live link would make the hit card a minefield. Images collapse
  // to their alt text, links unwrap to their label.
  for (const img of [...holder.querySelectorAll("img")]) {
    img.replaceWith(document.createTextNode(img.getAttribute("alt") ?? ""));
  }
  for (const anchor of [...holder.querySelectorAll("a")]) {
    anchor.replaceWith(...anchor.childNodes);
  }
  if (tokens.length > 0) markMatches(holder, tokens);

  return { html: holder.innerHTML, hasAbove: windowText.hasAbove, hasBelow: windowText.hasBelow };
}
