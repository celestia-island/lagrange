// The docs-site shell — the SAME hikari chrome the family's admin
// consoles wear (HkFaceShell + HkAdminHeader + HkSidebar/HkMenu), tuned
// for a static, account-free documentation site:
//
//   - header left wing: the site logo in HkAdminHeader's brand slot
//     (`brandSrc`, the zero-account branded face — no avatar, no
//     dropdown), followed by the page title;
//   - header right wing: the search field, then the top-right language
//     button (a direct button, per the 2026-11-22 user direction — NOT
//     the legacy bottom-left selector) and the theme button, which opens
//     a chest-anatomy settings window without the section rail: an
//     HkAdaptiveDialog whose only content is HkThemePanelContent;
//   - sidebar: the language's SUMMARY tree on HkMenu (variant=sidebar),
//     with the shell's rail collapse (expanded ↔ icon rail + hover
//     peek) persisted under lagrange's own storage namespace — the
//     library never persists it itself;
//   - footer: a single band — HkConnectionStatus (standalone, idle: a
//     static build has no connection to report) left, site title +
//     build stamp centre, "Built with Lagrange" right.
//
// Multi-page navigation is real page loads, not SPA routing: sidebar
// rows and locale picks assign location to the per-language URL the
// build emitted.
import { computed, defineComponent, ref, type PropType } from "vue";
import {
  HkAdaptiveDialog,
  HkAdminHeader,
  HkConnectionStatus,
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
      // External and anchor hrefs ride the SUMMARY verbatim — only
      // page paths get the language prefix.
      if (/^(https?:|mailto:|#)/.test(href)) {
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
    const localeOptions = computed(() => {
      const byCode = new Map(page.value.locales.map((l) => [l.code, l.url]));
      return site.value.langs.map((l) => ({
        code: l.code,
        label: l.label,
        url: byCode.get(l.code) ?? `${site.value.base}/${l.code}/`,
      }));
    });
    function onLocaleSelect(code: string) {
      const hit = localeOptions.value.find((l) => l.code === code);
      if (hit) window.location.assign(hit.url);
      localeOpen.value = false;
    }

    // ── theme button → rail-less settings dialog (theme only) ────────
    const themeOpen = ref(false);

    const footerZone = "lagrange-docs-footer";

    return () => (
      <HkFaceShell
        sidebarCollapsible
        sidebarRailStyle={railStyle.value}
        onUpdate:sidebarRailStyle={(v: string) => onRailStyleChange(v === "rail" ? "rail" : "expanded")}
        navTitle={t("lagrange::docs.navTitle", "Contents")}
        contentMode="document"
      >
        {{
          header: ({ compact }: FaceShellHeaderSlot) => (
            <HkAdminHeader
              compact={compact}
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
          footer: () => (
            <div class={footerZone}>
              <span class={`${footerZone}-left`}>
                {/* A static build has no live connection to report: the
                 * idle status line, never a fabricated "connected". */}
                <HkConnectionStatus standalone={false} idle={true} />
              </span>
              <span class={`${footerZone}-center`}>
                {site.value.title} · {site.value.buildStamp}
              </span>
              <span class={`${footerZone}-right`}>
                <a href="https://github.com/celestia-island/lagrange" target="_blank" rel="noreferrer">
                  Built with Lagrange
                </a>
              </span>
            </div>
          ),
        }}
      </HkFaceShell>
    );
  },
});
