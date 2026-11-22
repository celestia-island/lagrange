// The header's search field — the docs face of the family's top-right
// action area. Queries the build's sharded static index (DocsSearch)
// in the CURRENT page language and renders the hits as plain anchors
// (real links, open-in-new-tab works). The panel is position-anchored
// under the input, dismissed by Escape / picking / blurring.
import { computed, defineComponent, onBeforeUnmount, onMounted, ref, type PropType } from "vue";
import { useI18n } from "@celestia-island/hikari";
import type { DocsSiteMeta } from "./types";
import { DocsSearch, type Hit } from "./search";

export const DocsSearchBox = defineComponent({
  name: "LagrangeDocsSearchBox",
  props: {
    site: { type: Object as PropType<DocsSiteMeta>, required: true },
    lang: { type: String, required: true },
  },
  setup(props) {
    const { t } = useI18n();
    const query = ref("");
    const hits = ref<Hit[] | null>(null);
    const open = ref(false);
    const rootRef = ref<HTMLElement | null>(null);
    const search = computed(() => new DocsSearch(props.site, fetch));

    let seq = 0;
    async function run() {
      if (!query.value.trim()) {
        hits.value = null;
        open.value = false;
        return;
      }
      const token = (seq += 1);
      const results = await search.value.query(query.value, props.lang);
      if (token !== seq) return; // a newer keystroke already answered
      hits.value = results;
      open.value = true;
    }

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") open.value = false;
    }
    function onDocClick(e: MouseEvent) {
      if (rootRef.value && !rootRef.value.contains(e.target as Node)) open.value = false;
    }
    onMounted(() => {
      document.addEventListener("click", onDocClick);
      document.addEventListener("keydown", onKey);
    });
    onBeforeUnmount(() => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onKey);
    });

    return () => (
      <span class="lagrange-docs-search" ref={rootRef}>
        <input
          class="lagrange-docs-search-input"
          type="search"
          placeholder={t("lagrange::docs.search", "Search…")}
          autocomplete="off"
          spellcheck={false}
          value={query.value}
          onInput={(e: Event) => {
            query.value = (e.target as HTMLInputElement).value;
            void run();
          }}
          onFocus={() => {
            if (hits.value) open.value = true;
          }}
        />
        {open.value && hits.value && (
          <div class="lagrange-docs-search-results" role="listbox">
            {hits.value.length === 0 && (
              <div class="lagrange-docs-search-empty">
                {t("lagrange::docs.searchNoResults", "No results")}
              </div>
            )}
            {hits.value.map((hit) => (
              <a class="lagrange-docs-search-hit" key={hit.url} href={hit.url}>
                <span class="lagrange-docs-search-hit-title">{hit.title}</span>
                {hit.heading && <span class="lagrange-docs-search-hit-heading">{hit.heading}</span>}
                {hit.snippet && <span class="lagrange-docs-search-hit-snippet">{hit.snippet}</span>}
              </a>
            ))}
          </div>
        )}
      </span>
    );
  },
});
