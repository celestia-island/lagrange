// DpiSliderBlock — the display-scale (DPI) control for the docs shell,
// ported from shittim-chest's DpiSliderBlock (2026-11-25 user direction:
// the docs theme dialog gets the chest anatomy's DPI section, copied —
// "直接抄隔壁"). The slider section rides hikari's standard
// HkExpansionPanel, DEFAULT COLLAPSED: the header carries the title +
// the live value (自动 / N%), the slider, its actions and both hints
// live in the body and stay out of the way until opened. Rendered into
// HkThemePanelContent's `mode-extra` strip (mode-adjacent chrome, not a
// theme editor tab).
//
// The INTERACTION CONTRACT is chest's, unchanged: dragging only stages a
// value; 应用 previews it and starts the app-level countdown in dpiPrefs
// (the confirm modal is a pure view of that store — 保留 persists, "立即
// 还原" / expiry restores); 自动 resets to the browser's own zoom. The
// keyboard/URL hatches (Ctrl+Alt+0, ?dpi=auto) keep working with no DPI
// UI mounted at all. Unmounting mid-COUNTDOWN reverts the live preview
// immediately, and the block re-syncs its persisted value whenever the
// countdown settles, so an expiry or the global hatch that fired
// elsewhere still lands the slider on truth. The collapsed panel keeps
// its body MOUNTED (upstream panel contract), so staged values and the
// countdown survive folding.
import { computed, defineComponent, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "@celestia-island/hikari";
import {
  HkButton,
  HkExpansionPanel,
  HkModal,
  HkSlider,
  type ModalAction,
} from "@celestia-island/hikari";
import {
  DPI_MAX,
  DPI_MIN,
  DPI_STEP,
  isDpiRisky,
  keepDpiScale,
  loadDpiScale,
  previewDpiScale,
  resetDpiScale,
  revertPreviewDpiScale,
  useAppliedDpiScale,
  useDpiCountdown,
  onDpiPrefsChange,
} from "./dpiPrefs";

export default defineComponent({
  name: "LagrangeDocsDpiSliderBlock",
  setup() {
    const { t } = useI18n();
    const dpiScale = ref<number | null>(loadDpiScale());
    const pendingDpi = ref<number | null>(null);
    const dpiAuto = computed(() => pendingDpi.value == null && dpiScale.value == null);
    const dpiCountdown = useDpiCountdown();

    // hikari's t() has no interpolation slot (a flat-key lookup with an
    // English fallback), so the two parameterized sentences interpolate
    // the same way hikari's own groupCount label does: literal {token}
    // placeholders in the message, replaced here.
    const revertCountdownText = computed(() =>
      t("lagrange::docs.dpiRevertCountdown", "{scale}% applied — reverting in {seconds}s")
        .replaceAll("{scale}", String(dpiCountdown.scale ?? ""))
        .replaceAll("{seconds}", String(dpiCountdown.remaining)),
    );
    const riskyWarningText = computed(() =>
      t("lagrange::docs.dpiRiskyWarning", "Scaling to {scale}% may push the interface off-screen")
        .replaceAll("{scale}", String(pendingDpi.value ?? "")),
    );

    // The store is app-level truth, and the DRAWN scale is its most
    // complete witness: every write path — apply, keep, revert, the
    // expiry task, the global keyboard / URL hatches — repaints the
    // document root through applyScaleVisually, and the applied ref
    // tracks that DOM attribute. Re-read the persisted value whenever it
    // moves, so the header never shows a stale notch — INCLUDING the
    // no-preview case (Ctrl+Alt+0 while auto/persisted, where no
    // countdown ever ran and an active-edge watcher would stay silent).
    const appliedScale = useAppliedDpiScale();
    watch(appliedScale, () => {
      dpiScale.value = loadDpiScale();
    });
    // The store's own change notification — the witness that sees the
    // reset-to-Auto path even where the DOM-attribute observer is
    // silent (removeAttribute does not surface as a mutation on some
    // platforms).
    const unwatchPrefsChange = onDpiPrefsChange(() => {
      dpiScale.value = loadDpiScale();
    });
    onBeforeUnmount(unwatchPrefsChange);
    // Leaving the theme dialog mid-COUNTDOWN reverts the live preview
    // immediately (discard-on-close grammar): a preview whose confirm
    // surface just unmounted must not linger visually until the expiry
    // task answers.
    onBeforeUnmount(() => {
      if (dpiCountdown.active) revertPreviewDpiScale();
    });

    function onDpiScaleChange(value: number) {
      pendingDpi.value = value;
    }

    function onDpiApply() {
      const pct = pendingDpi.value;
      if (pct == null) return;
      pendingDpi.value = null;
      previewDpiScale(pct);
      dpiScale.value = loadDpiScale();
    }

    function onDpiAuto() {
      pendingDpi.value = null;
      resetDpiScale();
      dpiScale.value = null;
    }

    function onDpiRevertNow() {
      revertPreviewDpiScale();
    }

    const confirmActions = computed(() => [
      { label: t("lagrange::docs.dpiKeep", "Keep"), variant: "primary" as const, onClick: onDpiKeep },
      { label: t("lagrange::docs.dpiRevertNow", "Revert now"), variant: "secondary" as const, onClick: onDpiRevertNow },
    ]);

    function onDpiKeep() {
      keepDpiScale();
      dpiScale.value = loadDpiScale();
    }

    /** The header's live value: the staged (pending) notch wins — the
     *  header shows what WILL apply once the countdown confirms. */
    const valueText = computed(() => {
      const staged = pendingDpi.value ?? dpiScale.value;
      return staged == null ? t("lagrange::docs.dpiAuto", "Auto") : `${staged}%`;
    });

    return () => {
      const pending = pendingDpi.value;
      const warningVisible = pending != null && isDpiRisky(pending, window.innerWidth);
      return (
        <>
          <HkExpansionPanel
            class="lagrange-dpi-panel"
            title={t("lagrange::docs.dpiTitle", "Display scale (DPI)")}
            subtitle={valueText.value}
            data-auto={dpiAuto.value || undefined}
          >
            <div class="lagrange-dpi-body" role="group" aria-label={t("lagrange::docs.dpiTitle", "Display scale (DPI)")}>
              <HkSlider
                class="lagrange-dpi-slider"
                min={DPI_MIN}
                max={DPI_MAX}
                step={DPI_STEP}
                showTicks
                modelValue={pending ?? dpiScale.value ?? DPI_MIN}
                onUpdate:modelValue={onDpiScaleChange}
                ariaLabel={t("lagrange::docs.dpiTitle", "Display scale (DPI)")}
                formatValue={(v: number) => `${v}%`}
              />
              <div class="lagrange-dpi-actions">
                <HkButton
                  size="sm"
                  variant="secondary"
                  disabled={dpiAuto.value}
                  onClick={onDpiAuto}
                >
                  {t("lagrange::docs.dpiAuto", "Auto")}
                </HkButton>
                {pending != null && (
                  <HkButton size="sm" variant="primary" onClick={onDpiApply}>
                    {t("lagrange::docs.dpiApply", "Apply")}
                  </HkButton>
                )}
              </div>
              {warningVisible && (
                <p class="lagrange-dpi-warning">
                  {riskyWarningText.value}
                </p>
              )}
              <p class="lagrange-dpi-hint">{t("lagrange::docs.dpiHint", "Auto follows the browser's own zoom untouched; the slider fixes the whole scale in 25% steps from 100% to 300%.")}</p>
              <p class="lagrange-dpi-hint">{t("lagrange::docs.dpiResetHint", "Scaled too large to operate? Press Ctrl+Alt+0 to reset to Auto, or add ?dpi=auto to the address bar.")}</p>
            </div>
          </HkExpansionPanel>

          <HkModal
            modelValue={dpiCountdown.active}
            onUpdate:modelValue={(v: boolean) => {
              if (!v && dpiCountdown.active) onDpiKeep();
            }}
            title={t("lagrange::docs.dpiConfirmTitle", "Keep this scale")}
            width="22rem"
            footerActions={confirmActions.value}
          >
            <p class="lagrange-dpi-hint">
              {revertCountdownText.value}
            </p>
          </HkModal>
        </>
      );
    };
  },
});
