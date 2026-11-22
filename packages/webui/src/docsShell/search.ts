// Client search over the build's sharded index (search_meta.json +
// search_i_NN.json). Tokenization mirrors src/search.rs exactly — ASCII
// alphanumeric runs (≥2 chars, lowercased) plus CJK bigrams — and the
// shard hash is `charCodeAt(0) % 16`, the mapping pinned by the Rust
// side's self-test. A query loads only the shards its tokens live in.
import type { DocsSiteMeta } from "./types";

interface ChunkMeta {
  heading_path: string[];
  anchor: string;
  snippet: string;
}

interface DocMeta {
  id: number;
  lang: string;
  title: string;
  /** Flat URL from the build: `base + "/" + page_path` (no language segment). */
  url: string;
  document_id: string;
  chunks: ChunkMeta[];
}

interface SearchMeta {
  docs: DocMeta[];
  shards: string[];
}

export interface Hit {
  title: string;
  heading: string;
  url: string;
  snippet: string;
}

export const SHARD_COUNT = 16;

/** Mirror of search.rs `tokenize` — see the module doc. */
export function tokenizeQuery(text: string): string[] {
  const tokens: string[] = [];
  const chars = Array.from(text);
  let i = 0;
  const isCjk = (ch: string) => {
    const c = ch.codePointAt(0) ?? 0;
    return (
      (c >= 0x4e00 && c <= 0x9fff) ||
      (c >= 0x3040 && c <= 0x30ff) ||
      (c >= 0xac00 && c <= 0xd7af) ||
      (c >= 0x3400 && c <= 0x4dbf)
    );
  };
  while (i < chars.length) {
    const ch = chars[i];
    if (/[a-zA-Z0-9]/.test(ch)) {
      let word = "";
      while (i < chars.length && /[a-zA-Z0-9]/.test(chars[i])) {
        word += chars[i].toLowerCase();
        i += 1;
      }
      if (word.length >= 2) tokens.push(word);
    } else if (isCjk(ch)) {
      if (i + 1 < chars.length && isCjk(chars[i + 1])) {
        tokens.push(ch + chars[i + 1]);
      }
      i += 1;
    } else {
      i += 1;
    }
  }
  return tokens;
}

/** `shard_for` — charCodeAt(0) % SHARD_COUNT. */
export function shardFor(word: string): number {
  return (word.codePointAt(0) ?? 63) % SHARD_COUNT;
}

/** Insert the language segment into a flat build URL. */
export function langUrl(site: DocsSiteMeta, url: string, lang: string): string {
  if (!site.base) return `/${lang}${url}`;
  return `${site.base}/${lang}${url.slice(site.base.length)}`;
}

export class DocsSearch {
  private meta: SearchMeta | null = null;
  private shards = new Map<string, Record<string, number[][]>>();

  constructor(private site: DocsSiteMeta, private fetchImpl: typeof fetch) {}

  private async loadMeta(): Promise<SearchMeta | null> {
    if (this.meta) return this.meta;
    try {
      const res = await this.fetchImpl(`${this.site.base}/search_meta.json`);
      if (!res.ok) return null;
      this.meta = (await res.json()) as SearchMeta;
      return this.meta;
    } catch {
      return null;
    }
  }

  private async loadShard(name: string): Promise<Record<string, number[][]> | null> {
    const cached = this.shards.get(name);
    if (cached) return cached;
    try {
      const res = await this.fetchImpl(`${this.site.base}/${name}`);
      if (!res.ok) return null;
      const shard = ((await res.json()) as { index: Record<string, number[][]> }).index;
      this.shards.set(name, shard);
      return shard;
    } catch {
      return null;
    }
  }

  /** Query in `lang`: hits from that language's translations only. */
  async query(query: string, lang: string): Promise<Hit[]> {
    const meta = await this.loadMeta();
    if (!meta) return [];
    const tokens = tokenizeQuery(query);
    if (tokens.length === 0) return [];
    const langDocs = new Map<number, DocMeta>();
    for (const doc of meta.docs) {
      if (doc.lang === lang) langDocs.set(doc.id, doc);
    }
    if (langDocs.size === 0) return [];

    const byShard = new Map<number, Set<string>>();
    for (const token of tokens) {
      const idx = shardFor(token);
      if (!byShard.has(idx)) byShard.set(idx, new Set());
      byShard.get(idx)!.add(token);
    }

    // chunk key -> distinct matching tokens (rank), per doc.
    const chunkHits = new Map<string, { doc: DocMeta; chunk: number; score: number }>();
    for (const [idx, words] of byShard) {
      const shardName = meta.shards[idx];
      if (!shardName) continue;
      const shard = await this.loadShard(shardName);
      if (!shard) continue;
      for (const word of words) {
        const postings = shard[word];
        if (!postings) continue;
        for (const [docId, chunkIdx] of postings) {
          const doc = langDocs.get(docId);
          if (!doc) continue;
          const key = `${docId}:${chunkIdx}`;
          const prev = chunkHits.get(key);
          if (prev) prev.score += 1;
          else chunkHits.set(key, { doc, chunk: chunkIdx, score: 1 });
        }
      }
    }

    const hits: Hit[] = [...chunkHits.values()]
      .sort((a, b) => b.score - a.score)
      .slice(0, 8)
      .map(({ doc, chunk }) => {
        const meta3 = doc.chunks[chunk];
        const heading = meta3?.heading_path?.join(" › ") ?? "";
        const anchor = meta3?.anchor;
        return {
          title: doc.title,
          heading,
          // No trailing '#' when the chunk meta is absent — a bare hash
          // would read as "top of page" instead of a missing target.
          url: anchor
            ? `${langUrl(this.site, doc.url, doc.lang)}#${anchor}`
            : langUrl(this.site, doc.url, doc.lang),
          snippet: meta3?.snippet ?? "",
        };
      });
    return hits;
  }
}
