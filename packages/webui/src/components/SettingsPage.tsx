// Settings page: default-language select, site-title / content-width
// fields (JSX v-model through @vue/babel-plugin-jsx), and the About grid
// fed from KB build info. Formerly SettingsPage.vue — the SFC moved to
// TSX + a side SCSS file (family form); props contract (`kb`) and
// rendered tree unchanged. The scoped style block is reproduced nested
// under the component's own root class `.settings-page` (scoped-data-v
// semantics).
import { computed, defineComponent, ref, type PropType } from "vue";
import type { KnowledgeBase } from "../composables/types";
import "./SettingsPage.scss";

export default defineComponent({
  name: "SettingsPage",
  props: {
    kb: { type: Object as PropType<KnowledgeBase>, required: true },
  },
  setup(props) {
    const siteTitle = ref("Lagrange");
    const defaultLang = ref("en");
    const contentWidth = ref("800px");

    const langs = computed(() => props.kb.source.langs);
    const sourceCommit = computed(() => props.kb.source.source_commit ?? "unknown");
    const parserVersion = computed(() => `v${props.kb.parser_version}`);
    const schemaVersion = computed(() => `v${props.kb.schema_version}`);

    const buildInfo = computed(() => ({
      generator: props.kb.generator ?? "lagrange",
      sourceCommit: (props.kb.source?.source_commit ?? "unknown").slice(0, 12),
      parserVersion: `v${props.kb.parser_version}`,
      schemaVersion: `v${props.kb.schema_version}`,
      langs: props.kb.source?.langs?.join(", ") ?? "",
    }));

    return () => (
      <div class="settings-page">
        <h2>Settings</h2>

        <section class="settings-section">
          <h3>Language</h3>
          <p class="hint">Default language for the knowledge base.</p>
          <select v-model={defaultLang.value} class="settings-select">
            {langs.value.map((lang) => (
              <option key={lang} value={lang}>{lang}</option>
            ))}
          </select>
        </section>

        <section class="settings-section">
          <h3>Site</h3>
          <div class="settings-field">
            <label>Site title</label>
            <input v-model={siteTitle.value} type="text" class="settings-input" />
          </div>
          <div class="settings-field">
            <label>Content width</label>
            <input v-model={contentWidth.value} type="text" class="settings-input" />
          </div>
        </section>

        <section class="settings-section">
          <h3>About</h3>
          <div class="about-grid">
            <div class="about-item"><span class="about-label">Generator</span><span>{buildInfo.value.generator}</span></div>
            <div class="about-item"><span class="about-label">Source commit</span><span class="mono">{sourceCommit.value}</span></div>
            <div class="about-item"><span class="about-label">Parser</span><span>{parserVersion.value}</span></div>
            <div class="about-item"><span class="about-label">Schema</span><span>{schemaVersion.value}</span></div>
            <div class="about-item"><span class="about-label">Languages</span><span>{langs.value.join(", ")}</span></div>
          </div>
        </section>
      </div>
    );
  },
});
