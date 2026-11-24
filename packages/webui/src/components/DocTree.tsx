// The per-document tree under Build Diagnostics: one node per document
// with title, id, kind badge, tags, and the per-language revision list.
// Formerly DocTree.vue — the SFC moved to TSX + a side SCSS file (family
// form); props contract (`kb`) and rendered tree unchanged. The scoped
// style block is reproduced nested under the component's own root class
// `.doc-tree` (scoped-data-v semantics).
import { computed, defineComponent, type PropType } from "vue";
import type { KnowledgeBase } from "../composables/types";
import "./DocTree.scss";

interface TreeDoc {
  id: string;
  title: string;
  kind: string;
  tags: string[];
  langs: { code: string; title: string; path: string; revision: string }[];
}

export default defineComponent({
  name: "DocTree",
  props: {
    kb: { type: Object as PropType<KnowledgeBase>, required: true },
  },
  setup(props) {
    const tree = computed<TreeDoc[]>(() =>
      props.kb.documents.map((doc) => ({
        id: doc.document_id,
        title: doc.translations["en"]?.title ?? doc.document_id,
        kind: doc.kind,
        tags: doc.tags,
        langs: Object.entries(doc.translations)
          .map(([code, t]) => ({ code, ... t }))
          .sort((a, b) => a.code.localeCompare(b.code)),
      })),
    );

    return () => (
      <div class="doc-tree">
        {tree.value.map((doc) => (
          <div key={doc.id} class="doc-node">
            <div class="doc-header">
              <span class="doc-title">{doc.title}</span>
              <span class="doc-id">{doc.id}</span>
              <span class="doc-kind badge">{doc.kind}</span>
            </div>
            {doc.tags.length > 0 ? (
              <div class="doc-tags">
                {doc.tags.map((tag) => (
                  <span key={tag} class="tag">{tag}</span>
                ))}
              </div>
            ) : null}
            <div class="lang-list">
              {doc.langs.map((lang) => (
                <span key={lang.code} class="lang-item">
                  <span class="lang-code">{lang.code}</span>
                  <span class="lang-title">{lang.title}</span>
                  <span class="lang-rev" title={lang.revision}>{lang.revision.slice(0, 8)}</span>
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  },
});
