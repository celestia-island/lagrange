export interface ChunkSpan {
  start_line: number;
  end_line: number;
  start_byte: number;
  end_byte: number;
}

export interface Chunk {
  document_id: string;
  lang: string;
  revision: string;
  heading_path: string[];
  anchor: string;
  span: ChunkSpan;
  content_hash: string;
  title: string;
}

export interface DocumentInfo {
  document_id: string;
  kind: string;
  tags: string[];
  translations: Record<string, { path: string; title: string; revision: string }>;
}

export interface KnowledgeBase {
  schema_version: number;
  parser_version: string;
  generator: string;
  source: { root: string; langs: string[]; source_commit: string | null };
  documents: DocumentInfo[];
  revisions: Record<string, { content_hash: string; source_commit: string | null; parser_version: string }>;
  chunks: Chunk[];
}
