// The docs-site shell — the SAME hikari chrome the family's admin
// consoles wear (HkFaceShell + HkAdminHeader + HkSidebar/HkMenu), tuned
// for a static, account-free documentation site:
//
//   - header left wing: the site logo in HkAdminHeader's brand slot
//     (`brandSrc`, the zero-account branded face — no avatar, no
//     dropdown), followed by the page title;
//   - header right wing: the search, then the top-right language
//     button (a direct button, per the 2026-11-22 user direction — NOT
//     the legacy bottom-left selector) and the theme button, which opens
//     a chest-anatomy settings window without the section rail: an
//     HkAdaptiveDialog whose only content is HkThemePanelContent. The
//     search is an inline field on desktop and one icon button + bottom
//     sheet on mobile, so the end wing never overflows a phone (the
//     language button was cut off at the viewport edge, 2026-11-24);
//   - sidebar: the language's SUMMARY tree on HkMenu (variant=sidebar),
//     with the shell's rail collapse (expanded ↔ icon rail + hover
//     peek) persisted under lagrange's own storage namespace — the
//     library never persists it itself;
//   - footer: none. A static, account-free docs site has nothing to
//     report — no connection to show, no status to light — and on a
//     phone the band cost a sixth of the viewport (2026-11-24 user
//     direction: no indicator bottom-left, nothing bottom on demand).
//     The build stamp lives in each page's own content where it belongs.
//
// Multi-page navigation is real page loads, not SPA routing: sidebar
// rows and locale picks assign location to the per-language URL the
// build emitted.
import { computed, defineComponent, ref, type PropType } from "vue";
import {
  HkAdaptiveDialog,
  HkAdminHeader,
  HkFaceShell,
  HkIconButton,
  HkLocalePickerPopup,
  HkMenu,
  HkSidebar,
  HkThemePanelContent,
  provideMenuGroupState,
  useI18n,
  useChromeActionSize,
  type HkMenuItem,
} from "@celestia-island/hikari";
import { Languages, Moon } from "lucide-vue-next";
import type { DocsNavNode, DocsPageData } from "./types";
import { DocsSearchBox } from "./DocsSearchBox";
import "./docsShell.scss";

const RAIL_STORAGE_KEY = "lagrange.docs.sidebar";

interface FaceShellHeaderSlot {
  compact: boolean;
  onOpenDrawer: () => void;
}

interface FaceShellSidebarSlot {
  collapsed: boolean;
  onNavigate: () => void;
  peeking?: boolean;
  inDrawer?: boolean;
}

/** SUMMARY tree → HkMenu items: sections become collapsible groups,
 * leaves become rows keyed by their language-relative href. */
export function navToMenuItems(nav: DocsNavNode[]): HkMenuItem[] {
  return nav.map((node, i) => ({
    key: node.href || `section:${i}:${node.title}`,
    label: node.title,
    children: node.children?.length ? navToMenuItems(node.children) : undefined,
  }));
}

/**
 * Language-picker options: every site language, mapped to this page's
 * copy when it exists and to that language's root otherwise (the root
 * is itself a negotiation stub, so the pick always lands somewhere real
 * — never a dead URL). Exported so the fallback has an executing test;
 * every page in today's docs tree happens to exist in all languages, so
 * the fallback branch is otherwise unreachable (PR #119 round 4, M14).
 */
export function buildLocaleOptions(
  langs: { code: string; label: string }[],
  locales: { code: string; url: string }[],
  base: string,
): { code: string; label: string; url: string }[] {
  const byCode = new Map(locales.map((l) => [l.code, l.url]));
  return langs.map((l) => ({
    code: l.code,
    label: l.label,
    url: byCode.get(l.code) ?? `${base}/${l.code}/`.replace(/\/{2,}/g, "/"),
  }));
}

export const LagrangeDocsShell = defineComponent({
  name: "LagrangeDocsShell",
  props: {
    page: { type: Object as PropType<DocsPageData>, required: true },
  },
  setup(props) {
    // The nav GROUP FOLD store lives at this setup, above HkFaceShell's
    // rail swap (the swap remounts each density fresh; the fold must
    // outlive it) — the same one-call contract chest's AdminLayout has.
    provideMenuGroupState();
    const { t } = useI18n();
    const chromeSize = useChromeActionSize();

    const page = computed(() => props.page);
    const site = computed(() => props.page.site);

    // ── sidebar rail persistence (lagrange's own namespace) ───────────
    const railStyle = ref<"expanded" | "rail">("expanded");
    if (typeof window !== "undefined") {
      const saved = window.localStorage.getItem(RAIL_STORAGE_KEY);
      if (saved === "rail" || saved === "expanded") railStyle.value = saved;
    }
    function onRailStyleChange(v: "expanded" | "rail") {
      railStyle.value = v;
      try {
        window.localStorage.setItem(RAIL_STORAGE_KEY, v);
      } catch {
        // Private-browsing storage refusal is not worth breaking nav over.
      }
    }

    // ── sidebar navigation: real page loads ───────────────────────────
    const menuItems = computed(() => navToMenuItems(page.value.nav));
    const activeKey = computed(() => page.value.path);
    function go(href: string) {
      // External, anchor and root-absolute hrefs ride the SUMMARY
      // verbatim (the same passthrough set absolute_href allows) — only
      // page paths get the language prefix.
      if (/^(https?:|mailto:|tel:|#|\/)/.test(href)) {
        window.location.assign(href);
        return;
      }
      const base = site.value.base;
      window.location.assign(`${base}/${page.value.lang}/${href}`.replace(/\/{2,}/g, "/"));
    }
    function onSelect(key: string) {
      // Section headers carry synthetic keys; only leaf rows navigate.
      if (key.startsWith("section:")) return;
      go(key);
    }

    // ── top-right language button ─────────────────────────────────────
    const localeOpen = ref(false);
    const langButtonRef = ref<HTMLElement | null>(null);
    /** Languages this page exists in, plus the rest of the site's
     * languages routed to their index (a language with no translation
     * of THIS page still gets a reachable pick, never a dead one). */
    const localeOptions = computed(() =>
      buildLocaleOptions(site.value.langs, page.value.locales, site.value.base),
    );
    function onLocaleSelect(code: string) {
      const hit = localeOptions.value.find((l) => l.code === code);
      if (hit) window.location.assign(hit.url);
      localeOpen.value = false;
    }

    // ── theme button → rail-less settings dialog (theme only) ────────
    const themeOpen = ref(false);

    return () => (
      <HkFaceShell
        sidebarCollapsible
        sidebarRailStyle={railStyle.value}
        onUpdate:sidebarRailStyle={(v: string) => onRailStyleChange(v === "rail" ? "rail" : "expanded")}
        navTitle={t("lagrange::docs.navTitle", "Contents")}
        contentMode="document"
      >
        {{
          header: ({ compact, onOpenDrawer }: FaceShellHeaderSlot) => (
            <HkAdminHeader
              compact={compact}
              // Below the shell breakpoint the drawer is the ONLY nav surface,
              // and the branded face carries no avatar trigger to open it, so
              // the hamburger is the sole entry point (the docs plan rule).
              showHamburger={compact}
              onHamburger={() => onOpenDrawer()}
              avatarAction="menu"
              title={page.value.title}
              brandSrc={site.value.logo ?? undefined}
              brandAlt={site.value.title}
              actions={[
                <DocsSearchBox key="search" lang={page.value.lang} site={site.value} />,
              ]}
              v-slots={{
                "theme-toggle": () => (
                  <span class="lagrange-docs-header-end">
                    <span ref={langButtonRef as any}>
                      <HkIconButton
                        size={chromeSize.value}
                        variant="ghost"
                        aria-label={t("lagrange::docs.language", "Language")}
                        {...{ title: t("lagrange::docs.language", "Language") }}
                        onClick={() => {
                          localeOpen.value = !localeOpen.value;
                        }}
                      >
                        {{ icon: () => <Languages size={16} /> }}
                      </HkIconButton>
                    </span>
                    <HkIconButton
                      size={chromeSize.value}
                      variant="ghost"
                      aria-label={t("lagrange::docs.theme", "Theme")}
                      {...{ title: t("lagrange::docs.theme", "Theme") }}
                      onClick={() => {
                        themeOpen.value = true;
                      }}
                    >
                      {{ icon: () => <Moon size={16} /> }}
                    </HkIconButton>
                    <HkLocalePickerPopup
                      open={localeOpen.value}
                      onUpdate:open={(v: boolean) => {
                        localeOpen.value = v;
                      }}
                      triggerRef={langButtonRef.value}
                      locales={localeOptions.value.map((l) => ({
                        code: l.code,
                        label: l.label,
                      }))}
                      currentLocale={page.value.lang}
                      t={(k: string) => t(k)}
                      onSelect={onLocaleSelect}
                    />
                    {/* The chest settings window's anatomy minus the
                     * section rail: one dialog, one pane, theme only. */}
                    <HkAdaptiveDialog
                      modelValue={themeOpen.value}
                      title={t("lagrange::docs.theme", "Theme")}
                      onUpdate:modelValue={(v: boolean) => {
                        themeOpen.value = v;
                      }}
                    >
                      <HkThemePanelContent />
                    </HkAdaptiveDialog>
                  </span>
                ),
              }}
            />
          ),
          sidebar: ({ collapsed, inDrawer }: FaceShellSidebarSlot) => (
            <HkSidebar width={inDrawer ? "100%" : "248px"} collapsed={collapsed}>
              {{
                default: () => (
                  <HkMenu
                    variant="sidebar"
                    open={true}
                    items={menuItems.value}
                    activeKey={activeKey.value}
                    defaultExpandGroups
                    collapsed={collapsed}
                    title={t("lagrange::docs.navTitle", "Contents")}
                    onSelect={onSelect}
                  />
                ),
              }}
            </HkSidebar>
          ),
          content: () => (
            <article class="lagrange-docs-content">
              {/* renderMode html: the build's pre-rendered body (the
               * no-JS baseline keeps every lagrange build-time feature);
               * markdown mode is the hikari/tairitsu renderer lane and
               * arrives with its upstream SSR seed. */}
              <div class="lagrange-docs-body" innerHTML={page.value.html} />
            </article>
          ),
        }}
      </HkFaceShell>
    );
  },
});
