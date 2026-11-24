// Search seam tests: the client tokenizer + shard hash MUST mirror
// src/search.rs (its self-test pins 'l'→12, 'd'→4, 'a'→1).
// MUT: M14 (read the shard as `.index`).
// The fixtures carry the BARE postings map — the shape the build writes.

import { describe, expect, it } from "vitest";
import { DocsSearch, langUrl, shardFor, tokenizeQuery } from "../search";
import type { DocsSiteMeta } from "../types";

describe("tokenizeQuery (mirror of search.rs tokenize)", () => {
  it("lowercases ASCII alphanumeric runs and drops single letters", () => {
    expect(tokenizeQuery("Lagrange Docs!")).toEqual(["lagrange", "docs"]);
    expect(tokenizeQuery("a b cd")).toEqual(["cd"]);
  });

  it("builds CJK bigrams", () => {
    expect(tokenizeQuery("目录结构")).toEqual(["目录", "录结", "结构"]);
  });

  it("returns nothing for punctuation-only queries", () => {
    expect(tokenizeQuery("?! —")).toEqual([]);
  });
});

describe("shardFor (pinned by search.rs self-test)", () => {
  it("maps first codepoint mod 16", () => {
    expect(shardFor("lagrange")).toBe(12);
    expect(shardFor("dir")).toBe(4);
    expect(shardFor("anchor")).toBe(1);
    expect(shardFor("目录")).toBe("目".codePointAt(0)! % 16);
  });
});

describe("langUrl", () => {
  const site = (base: string): DocsSiteMeta => ({
    title: "t",
    description: "d",
    logo: null,
    base,
    defaultLang: "en",
    langs: [],
    buildStamp: "s",
  });

  it("inserts the language segment after the base", () => {
    expect(langUrl(site(""), "/guides/a.html", "zh-Hans")).toBe("/zh-Hans/guides/a.html");
    expect(langUrl(site("/repo"), "/repo/guides/a.html", "en")).toBe("/repo/en/guides/a.html");
  });
});

describe("DocsSearch", () => {
  function makeFetcher(files: Record<string, unknown>) {
    return (async (url: string) => {
      const hit = files[url];
      if (!hit) return { ok: false, json: async () => ({}) };
      return { ok: true, json: async () => hit };
    }) as unknown as typeof fetch;
  }

  const meta = {
    docs: [
      {
        id: 0,
        lang: "en",
        title: "Architecture",
        url: "/guides/architecture.html",
        document_id: "guides/architecture",
        chunks: [{ heading_path: ["Architecture"], anchor: "a1", snippet: "the pipeline" }],
      },
      {
        id: 1,
        lang: "zh-Hans",
        title: "架构",
        url: "/guides/architecture.html",
        document_id: "guides/architecture",
        chunks: [{ heading_path: ["架构"], anchor: "a1", snippet: "管线" }],
      },
    ],
    shards: ["search_i_00.json", "search_i_01.json"],
  };

  it("queries only the requested language and rewrites URLs per-language", async () => {
    const s = new DocsSearch(
      { title: "t", description: "d", logo: null, base: "", defaultLang: "en", langs: [], buildStamp: "s" },
      makeFetcher({
        "/search_meta.json": meta,
        // 'a' = 97 % 16 = 1 → shard 01 holds "anchor". The shard is the
        // BARE postings map — the shape `src/search.rs` writes (its own
        // test pins it). A wrapped `{index: …}` fixture here is what let
        // the reader and the writer drift apart while every test stayed
        // green.
        "/search_i_01.json": { anchor: [[1, 0]] },
      }),
    );
    const hits = await s.query("anchor", "zh-Hans");
    expect(hits).toHaveLength(1);
    expect(hits[0].title).toBe("架构");
    expect(hits[0].url).toBe("/zh-Hans/guides/architecture.html#a1");
  });

  it("returns empty for unknown tokens instead of loading everything", async () => {
    const s = new DocsSearch(
      { title: "t", description: "d", logo: null, base: "", defaultLang: "en", langs: [], buildStamp: "s" },
      makeFetcher({ "/search_meta.json": meta, "/search_i_00.json": {} }),
    );
    expect(await s.query("zzzz", "en")).toEqual([]);
  });
});
