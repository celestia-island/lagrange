// The header's search — ONE shape on every viewport, per user direction
// (2026-11-24, twice refined after seeing the deployed face): a Search
// icon button in the SAME slot the language and theme buttons occupy,
// opening a SECOND-LEVEL WINDOW (HkModal — the family's global-search
// grammar, the same surface shittim-chest uses) that holds the field and
// the hits. Two earlier drafts were rejected by the user: a permanently
// visible inline field (first), then an anchored popover on desktop
// (second — "仍然不是直接打开二级窗口"). The header therefore stays a
// compact action row, and the search gets a real dialog.
//
// The FIELD is the upstream standard part (2026-11-25 user direction:
// the hand-rolled input+× pair was visibly not a library component) —
// HkSearchInput, the same search box every family surface uses: leading
// glyph, the family's input chrome, and the built-in clear ✕ with its
// own refocus. Its undeclared attributes land on the FIELD element (the
// HkInput contract), so the focus wiring rides through.
//
// The modal's TITLE and the trigger's accessible name are nouns WITHOUT
// an ellipsis (2026-11-25 user direction — "标题那个搜索后面不要带省略
// 号"): `searchTitle` ("Search" / "搜索"). The FIELD's placeholder keeps
// the typing affordance (`search` — "Search…" / "搜索…", the same
// convention hikari's own HkSearchInput placeholder ships).
//
// Queries the build's sharded static index (DocsSearch) in the CURRENT
// page language and renders the hits as plain anchors (real links,
// open-in-new-tab works).
//
// Dismissal is the modal's alone — Escape, an outside click, its
// scrim, its ✕ — all arriving as `update:modelValue(false)`. This
// component registers NO listener of its own: a `focusout` containment
// test made clicking a result impossible, because the panel is teleported
// outside the field's box (round 5 of #124). The async query path has an
// explicit error state — a rejected fetch or a corrupt shard must not
// leave the panel frozen on stale results or surface as an unhandled
// rejection (PR #119 round 5).
import {
  computed,
  defineComponent,
  onMounted,
  ref,
  watchEffect,
  type PropType,
} from "vue";
import { HkIconButton, HkModal, HkSearchInput, HkSpinner, useI18n } from "@celestia-island/hikari";
import { Search } from "lucide-vue-next";
import type { DocsSiteMeta } from "./types";
import { DocsSearch, tokenizeQuery, type Hit } from "./search";
import { buildSnippetView } from "./snippet";

export const DocsSearchBox = defineComponent({
  name: "LagrangeDocsSearchBox",
  props: {
    site: { type: Object as PropType<DocsSiteMeta>, required: true },
    lang: { type: String, required: true },
    /** The header's action size, passed down by the shell. hikari's
     * `useChromeActionSize` keeps its OWN `useBreakpoint` instance (its own
     * width snapshot), so a second instance here read a different width on
     * a cold phone load and the trigger rendered 28px beside two 32px
     * neighbours — at rest, not just first paint (round 15 of #124).
     * One source, one row. */
    size: { type: Number as PropType<28 | 32>, default: 32 },
  },
  setup(props) {
    const { t } = useI18n();
    // NOTHING inside the shell renders before mount, on the server or on
    // the client's first pass: SSR has no viewport, and the trigger's
    // action size is viewport-derived — a live value on the first client
    // pass mismatches the server's. Gating every child on `mounted` makes
    // the two first passes identical (an empty shell), and the mounted
    // pass paints once (rounds 3 and 5 of #124).
    const mounted = ref(false);
    const query = ref("");
    const hits = ref<Hit[] | null>(null);
    /** Results are on screen (desktop: the anchored panel; mobile: the
     * sheet). On mobile this IS the sheet's open state. */
    const open = ref(false);
    /** A query was attempted and the index could not be read. */
    const failed = ref(false);
    /** The panel has something to show: a query in flight, an answer, or
     * an error. The popover mounts on THIS, so the in-flight row is on
     * screen from the first keystroke — a popover keyed on `open` alone
     * mounted only once an answer landed, and the first query showed
     * nothing at all (round 4 of #124). */
    const panelUp = computed(() => busy.value || open.value);
    /** A query is in flight (kept out of the panel: the previous hits
     * stay visible until the new answer lands). */
    const busy = ref(false);
    const rootRef = ref<HTMLElement | null>(null);
    // `fetch` must be bound to `window`: DocsSearch stores it on an
    // instance field and calls it as a method, which makes `this` the
    // DocsSearch instance — and Chrome throws "Illegal invocation" for a
    // Window method invoked off the Window. DocsSearch's catch turned that
    // into null, so every query answered "unavailable" on every browser
    // while happy-dom (which does not check `this`) kept the tests green.
    const search = computed(() => new DocsSearch(props.site, fetch.bind(window)));

    let seq = 0;

    function close() {
      open.value = false;
    }

    function clear() {
      query.value = "";
      hits.value = null;
      failed.value = false;
      seq += 1; // invalidate any in-flight query
      // The invalidated run's `finally` will NOT reset `busy` (it checks
      // the token), so reset it here — otherwise `busy` leaks true and
      // `panelUp` pins an empty popover on screen (round 5 of #124).
      busy.value = false;
      // `open` stays TRUE: the surface is a modal now, and a modal that
      // closes itself the moment the field empties is unusable — the user
      // is still standing in front of it (round 7 of #124's rule, which
      // the modal makes universal: nothing to fold away, no invisible
      // backdrop to trap a header click).
      // Refocusing is HkSearchInput's own clear contract: its ✕ hands
      // focus back to the field, so a clear keeps typing alive without
      // this component holding an input ref.
    }

    async function run() {
      // Bump the token on EVERY run, including the clear path: a query
      // that resolves after the box was emptied must not repopulate it.
      const token = (seq += 1);
      failed.value = false;
      // A superseded run leaves `busy` set (its `finally` skips the reset
      // because the token moved), so every run starts from a quiet state
      // and raises the flag itself.
      busy.value = false;
      if (!query.value.trim()) {
        hits.value = null;
        // The modal stays open on every viewport (see `clear()`).
        return;
      }
      busy.value = true;
      try {
        const results = await search.value.query(query.value, props.lang);
        if (token !== seq) return; // a newer keystroke already answered
        hits.value = results;
        open.value = true;
      } catch {
        if (token !== seq) return;
        hits.value = [];
        failed.value = true;
        open.value = true;
      } finally {
        if (token === seq) busy.value = false;
      }
    }

    // Dismissal is the MODAL's, not ours: HkModal owns Escape, the
    // backdrop and its own ✕, and it teleports its surface outside
    // `rootRef` — the containment-test trap of round 5 of #124 (a click on
    // a hit read as "outside" and closed the surface before `click` could
    // navigate) cannot recur, because nothing here tests containment.
    function onModalUpdate(v: boolean) {
      if (v) return;
      close();
      // A dismissal must also CANCEL the in-flight query. Otherwise its
      // answer lands, `run()` sets `open = true`, and the panel comes back
      // on its own — the user closed it and it reappeared (round 14 of
      // #124, measured on a phone with the query held mid-flight). Same
      // two resets `clear()` does, for the same reason: the superseded
      // run's `finally` skips both once the token moves.
      seq += 1;
      busy.value = false;
    }

    onMounted(() => {
      mounted.value = true;
    });

    // The shell's state attributes are written imperatively, after mount:
    // binding them in the template would emit a shape from the server (no
    // viewport there) that the client would then contradict — an attribute
    // mismatch on every load. They exist for tests and the Chrome checks,
    // not for styling: the trigger is the only shape now, and `open` /
    // `busy` are the modal's state.
    watchEffect(() => {
      const el = rootRef.value;
      if (!el || !mounted.value) return;
      el.dataset.shape = "trigger";
      el.dataset.open = open.value ? "true" : "false";
      el.dataset.busy = busy.value ? "true" : "false";
    });

    const results = () => (
      <>
        {/* In-flight row: its OWN surface, so it never reads as the
            results panel (the "no empty panel flash" contract still
            holds — that assertion is what caught this overlap). */}
        {busy.value && !hits.value && !failed.value && (
          <div
            class="lagrange-docs-search-busy"
            role="status"
            aria-label={t("lagrange::docs.searchBusy", "Searching…")}
          >
            <HkSpinner size="sm" center />
          </div>
        )}
        {open.value && (hits.value || failed.value) && (
          <div class="lagrange-docs-search-results" role="listbox">
            {failed.value && (
              <div class="lagrange-docs-search-error" role="status">
                {t("lagrange::docs.searchFailed", "Search is unavailable right now")}
              </div>
            )}
            {!failed.value && hits.value?.length === 0 && (
              <div class="lagrange-docs-search-empty">
                {t("lagrange::docs.searchNoResults", "No results")}
              </div>
            )}
            {(hits.value ?? []).map((hit) => {
              // The magnifier view: a line-snapped window of the chunk's real
              // text around the first match, rendered AS markdown (pipe rows
              // become tables), matches underlined in place, edges faded only
              // where the section continues.
              const view = buildSnippetView(hit.text, query.value);
              return (
                <a class="lagrange-docs-search-hit" key={hit.url} href={hit.url}>
                  <span class="lagrange-docs-search-hit-title">{hit.title}</span>
                  {hit.heading && <span class="lagrange-docs-search-hit-heading">{hit.heading}</span>}
                  {view.html && (
                    <span
                      class={[
                        "lagrange-docs-search-hit-snippet",
                        view.hasAbove && "has-fade-above",
                        view.hasBelow && "has-fade-below",
                      ]}
                      // Sanitized in buildSnippetView (DOMPurify + a text-node
                      // walk that wraps matches in <mark>). innerHTML is a
                      // Vue recognized prop (ReservedProps) — the JSX type
                      // name for the Vue-2-era domPropsInnerHTML spelling.
                      {...{ innerHTML: view.html }}
                    />
                  )}
                </a>
              );
            })}
          </div>
        )}
      </>
    );

    // ONE structure for both viewports: the header holds the trigger and
    // the results live in an HkModal — the family's second-level window
    // (shittim-chest's global search wears exactly this surface). A modal,
    // not an anchored popover: the user asked twice for a real dialog, and
    // the modal brings the family's dismissal grammar (Escape, backdrop,
    // its own ✕) and a surface wide enough for the field plus hits.
    return () => (
      <span class="lagrange-docs-search" ref={rootRef}>
        {mounted.value && (
          <>
            <HkIconButton
              size={props.size}
              variant="ghost"
              aria-label={t("lagrange::docs.searchTitle", "Search")}
              {...{ title: t("lagrange::docs.searchTitle", "Search") }}
              aria-haspopup="dialog"
              aria-expanded={open.value ? "true" : "false"}
              onClick={() => {
                open.value = true;
              }}
            >
              {{ icon: () => <Search size={16} /> }}
            </HkIconButton>
            <HkModal
              modelValue={panelUp.value}
              onUpdate:modelValue={onModalUpdate}
              title={t("lagrange::docs.searchTitle", "Search")}
              width="36rem"
              contentClass="lagrange-docs-search-modal"
            >
              <div class="lagrange-docs-search-panel">
                <HkSearchInput
                  modelValue={query.value}
                  onUpdate:modelValue={(v: string) => {
                    query.value = v;
                    void run();
                  }}
                  onClear={clear}
                  placeholder={t("lagrange::docs.search", "Search…")}
                  aria-label={t("lagrange::docs.searchTitle", "Search")}
                  clearAriaLabel={t("lagrange::docs.clearSearch", "Clear search")}
                  // Enter in a live-search box must not wait on the
                  // debounce path the standard component offers — the
                  // query already ran on every keystroke.
                  debounce={0}
                />
                {results()}
              </div>
            </HkModal>
          </>
        )}
      </span>
    );
  },
});
