import ThemePreview from "@/components/ThemePreview"
import { i18n, Trans } from "@gd/i18n"
import { getThemeKey } from "@gd/i18n/helpers"
import { Show, JSX } from "solid-js"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import type { FESeasonStatus } from "@gd/core_module/bindings"
import {
  getBuiltinSeasonWindow,
  isSeasonalBuiltin,
  type BuiltinThemeId
} from "@/themes/builtin"
import { formatMonthDay, isThemeLocked, lockedDates } from "@/themes/season"
import { applyThemeByName, getThemeColor } from "@/utils/theme"

interface Props {
  theme: BuiltinThemeId
  /** The saved theme's settings key. */
  savedKey: string
  /** `themes.getSeason`'s answer; undefined while it loads or if it failed. */
  season: FESeasonStatus | undefined
  onSelect: (key: string) => void
}

const BuiltinThemeTile = (props: Props) => {
  const theme = props.theme
  const shade1 = getThemeColor(theme, "darkSlate-900")
  const shade2 = getThemeColor(theme, "darkSlate-700")
  const shade3 = getThemeColor(theme, "darkSlate-600")

  // The core decides availability; the renderer never compares dates. While
  // the status loads (or if it failed), only the seasonal built-ins are held
  // back, and their tooltip shows the manifest's window.
  const locked = () => isThemeLocked(theme, props.season, isSeasonalBuiltin)
  const dates = () => lockedDates(theme, props.season, getBuiltinSeasonWindow)
  const language = () => i18n.language || "english"

  const tile = (): JSX.Element => (
    <div
      data-testid={`theme-tile-${theme}`}
      data-locked={locked() ? "true" : undefined}
      class="w-42 relative rounded-md p-2 transition-transform duration-300 ease-spring"
      classList={{
        "hover:scale-105": !locked(),
        "opacity-60": locked(),
        "scale-105": props.savedKey === theme
      }}
      style={{ "background-color": shade1 }}
      onClick={() => {
        if (locked()) return
        props.onSelect(theme)
      }}
      onMouseEnter={() => {
        if (locked()) return
        applyThemeByName(theme, { preview: true })
      }}
      onMouseLeave={() => {
        if (locked()) return
        applyThemeByName(props.savedKey)
      }}
    >
      <div
        class="bg-darkSlate-500 absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full p-2 opacity-0 shadow-md transition-opacity duration-300 ease-spring"
        classList={{
          "opacity-100": locked() || props.savedKey === theme
        }}
      >
        <div
          class="h-6 w-6"
          classList={{
            "i-hugeicons:square-lock-02": locked(),
            "i-hugeicons:tick-02": !locked()
          }}
        />
      </div>

      <ThemePreview shade1={shade1} shade2={shade2} shade3={shade3} />
    </div>
  )

  return (
    <div>
      <Show when={dates()} fallback={tile()}>
        {(s) => (
          <Tooltip placement="top">
            <TooltipTrigger as="div" tabIndex={0}>
              {tile()}
            </TooltipTrigger>
            <TooltipContent>
              {i18n.t("settings:_trn_theme_available_dates", {
                from: formatMonthDay(s().from, language()),
                until: formatMonthDay(s().until, language()),
                interpolation: { escapeValue: false }
              })}
            </TooltipContent>
          </Tooltip>
        )}
      </Show>
      <div class="box-border flex w-full items-center justify-start gap-2 px-2 py-4">
        <p class="text-lightSlate-700 m-0">
          <Trans key={getThemeKey(theme)} />
        </p>
      </div>
    </div>
  )
}

export default BuiltinThemeTile
