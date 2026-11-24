// The header's search — the docs face of the family's top-right action
// area, in the two shapes the family's chrome wears:
//
//   - desktop (≥ the shell's mobile breakpoint): the inline field in the
//     header's end wing, its results in a panel anchored under the field;
//   - mobile: a single Search icon button — the SAME 32/24px slot the
//     language and theme buttons occupy — opening a bottom sheet
//     (HkPopover's `sheetOnMobile`) that holds the field and the hits.
//
// Why two shapes: an always-present field is a fixed-width resident in
// the end wing, and on a phone it squeezed the language and theme
// buttons past the viewport edge (2026-11-24: the language button was
// cut off at the right edge). The family's own search affordances
// (HkSearchTrigger) collapse to a trigger + sheet for exactly this
// reason; the docs search keeps an inline desktop field because a docs
// site is read more than it is acted on, and a visible field is the
// affordance readers look for.
//
// Queries the build's sharded static index (DocsSearch) in the CURRENT
// page language and renders the hits as plain anchors (real links,
// open-in-new-tab works).
//
// Dismissal: Escape, an outside click, or blurring the field. Every one
// of those routes through `close()`, and the async query path has an
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
import { HkIconButton, HkPopover, useBreakpoint, useChromeActionSize, useI18n } from "@celestia-island/hikari";
import { Search } from "lucide-vue-next";
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
    const { isMobile: breakpointMobile } = useBreakpoint();
    // NOTHING inside the shell renders before mount, on the server or on
    // the client's first pass: SSR has no viewport, so any shape it picks
    // is a guess the client must then take back, and the two shapes are
    // different elements (a field vs an icon button) — a swap, which Vue
    // reports as a hydration mismatch and resolves by replacing the server
    // subtree. Gating every child on `mounted` makes the two first passes
    // identical (an empty shell), and the mounted pass paints the right
    // shape once (rounds 3 and 5 of #124).
    const mounted = ref(false);
    const isMobile = computed(() => breakpointMobile.value);
    const chromeSize = useChromeActionSize();
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
    const inputRef = ref<HTMLInputElement | null>(null);
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
      inputRef.value?.focus();
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
        // Desktop: an emptied field folds its panel. Mobile: the sheet
        // stays up — the user is still typing into it, and a sheet that
        // closed on each backspace would be unusable.
        if (!isMobile.value) close();
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

    // Dismissal is HkPopover's, not ours. The panel is teleported outside
    // `rootRef`, so a containment test against `rootRef` called a search
    // hit "outside" — mousedown on a hit blurred the input, the panel
    // closed, and the anchor was gone before `click` could navigate
    // (round 5 of #124). HkPopover's own shield exempts its anchor AND its
    // panel, handles Escape, and owns the phone's backdrop. The family's
    // HkSearchTrigger registers no listener of its own for the same reason.
    function onSheetUpdate(v: boolean) {
      if (!v) close();
    }

    onMounted(() => {
      mounted.value = true;
    });

    // The shell's state attributes are written imperatively, after mount:
    // binding them in the template would emit `data-shape="inline"` from
    // the server (no viewport there) and `data-shape="sheet"` from a
    // phone's first client pass — an attribute mismatch on every load.
    // They exist for tests and the Chrome checks, not for styling.
    watchEffect(() => {
      const el = rootRef.value;
      if (!el || !mounted.value) return;
      el.dataset.shape = isMobile.value ? "sheet" : "inline";
      el.dataset.open = open.value ? "true" : "false";
      el.dataset.busy = busy.value ? "true" : "false";
    });

    const field = () => (
      <span class="lagrange-docs-search-field">
      <input
        class="lagrange-docs-search-input"
        ref={inputRef}
        type="search"
        placeholder={t("lagrange::docs.search", "Search…")}
        aria-label={t("lagrange::docs.search", "Search…")}
        autocomplete="off"
        spellcheck={false}
        // No autofocus: HkPopover focuses its own panel on the tick after
        // it opens and would take focus straight back, so the attribute
        // bought nothing (measured in Chrome, round 1 of #124).
        value={query.value}
        onInput={(e: Event) => {
          query.value = (e.target as HTMLInputElement).value;
          void run();
        }}
        onFocus={() => {
          if (hits.value) open.value = true;
        }}
      />
        {query.value && (
          <button
            class="lagrange-docs-search-clear"
            type="button"
            aria-label={t("lagrange::docs.clearSearch", "Clear search")}
            title={t("lagrange::docs.clearSearch", "Clear search")}
            onClick={clear}
          >
            ×
          </button>
        )}
      </span>
    );

    const results = () => (
      <>
        {/* In-flight row: its OWN surface, so it never reads as the
            results panel (the "no empty panel flash" contract still
            holds — that assertion is what caught this overlap). */}
        {busy.value && !hits.value && !failed.value && (
          <div class="lagrange-docs-search-busy" role="status">
            {t("lagrange::docs.searchBusy", "Searching…")}
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
            {(hits.value ?? []).map((hit) => (
              <a class="lagrange-docs-search-hit" key={hit.url} href={hit.url}>
                <span class="lagrange-docs-search-hit-title">{hit.title}</span>
                {hit.heading && <span class="lagrange-docs-search-hit-heading">{hit.heading}</span>}
                {hit.snippet && <span class="lagrange-docs-search-hit-snippet">{hit.snippet}</span>}
              </a>
            ))}
          </div>
        )}
      </>
    );

    // ONE structure for both shapes: the header holds the trigger (the
    // inline field on desktop, the icon button on a phone) and the results
    // live in an HkPopover teleported to the overlay root. The header's end
    // wing clips (`overflow: hidden`, hikari's own), so an in-place results
    // list was invisible on desktop — round 3 of #124 measured a panel
    // that existed in the layout and never reached the screen. HkPopover
    // gives desktop an anchored panel and the phone a bottom sheet from
    // the same element.
    return () => (
      <span class="lagrange-docs-search" ref={rootRef}>
        {mounted.value && (
          <>
            {isMobile.value ? (
              <HkIconButton
                size={chromeSize.value}
                variant="ghost"
                aria-label={t("lagrange::docs.search", "Search…")}
                {...{ title: t("lagrange::docs.search", "Search…") }}
                aria-haspopup="dialog"
                aria-expanded={open.value ? "true" : "false"}
                onClick={() => {
                  open.value = !open.value;
                }}
              >
                {{ icon: () => <Search size={16} /> }}
              </HkIconButton>
            ) : (
              field()
            )}
            <HkPopover
              modelValue={panelUp.value}
              onUpdate:modelValue={onSheetUpdate}
              // `.value`, the family's shape (HkColorPicker, HkAdminHeader,
              // …): the ref is written during the first patch, which
              // re-runs this render, and HkPopover reads the anchor only
              // once `modelValue` turns true — by then it is populated.
              anchorRef={rootRef.value}
              placement="bottom-end"
              offset={6}
              sheetOnMobile
              title={t("lagrange::docs.search", "Search…")}
            >
              {isMobile.value ? (
                <div class="lagrange-docs-search-sheet">
                  {field()}
                  {results()}
                </div>
              ) : (
                results()
              )}
            </HkPopover>
          </>
        )}
      </span>
    );
  },
});
