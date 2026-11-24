// Build-diagnostics page: KB statistics cards, build info, and the
// document tree. Formerly ConsolePage.vue — the SFC moved to TSX + a
// side SCSS file (family form); props contract (`kb`) and rendered tree
// unchanged. The scoped style block is reproduced nested under the
// component's own root class `.console-page` (scoped-data-v semantics).
import { computed, defineComponent, type PropType } from "vue";
import type { KnowledgeBase } from "../composables/types";
import DocTree from "./DocTree";
import "./ConsolePage.scss";

export default defineComponent({
  name: "ConsolePage",
  props: {
    kb: { type: Object as PropType<KnowledgeBase>, required: true },
  },
  setup(props) {
    const docStats = computed(() => {
      const docs = props.kb.documents;
      const totalTranslations = docs.reduce((acc, d) => acc + Object.keys(d.translations).length, 0);
      const totalChunks = props.kb.chunks.length;
      const langs = [...new Set(docs.flatMap((d) => Object.keys(d.translations)))].sort();
      return { docCount: docs.length, translationCount: totalTranslations, chunkCount: totalChunks, langs };
    });

    const buildInfo = computed(() => ({
      schemaVersion: props.kb.schema_version,
      parserVersion: props.kb.parser_version,
      generator: props.kb.generator,
      sourceCommit: props.kb.source.source_commit ?? "not in a git checkout",
    }));

    return () => (
      <div class="console-page">
        <h2>Build Diagnostics</h2>
        <div class="stat-grid">
          <div class="stat-card">
            <div class="stat-value">{docStats.value.docCount}</div>
            <div class="stat-label">Documents</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">{docStats.value.translationCount}</div>
            <div class="stat-label">Translations</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">{docStats.value.chunkCount}</div>
            <div class="stat-label">Chunks</div>
          </div>
        </div>
        <div class="stat-grid">
          <div class="stat-card">
            <div class="stat-label">Languages</div>
            <div class="stat-detail">{docStats.value.langs.join(", ")}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">Source commit</div>
            <div class="stat-detail">{buildInfo.value.sourceCommit}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">Parser</div>
            <div class="stat-detail">v{buildInfo.value.parserVersion} (schema v{buildInfo.value.schemaVersion})</div>
          </div>
        </div>
        <h2>Document Tree</h2>
        <DocTree kb={props.kb} />
      </div>
    );
  },
});
