import ThemePreview from "@/components/ThemePreview"
import { useTransContext } from "@gd/i18n"
import { createSignal, Show } from "solid-js"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import type { InstalledTileModel } from "@/themes/installed"
import { applyThemeByName } from "@/utils/theme"

interface Props {
  /** The theme's folder name. */
  id: string
  model: InstalledTileModel
  /** The saved theme's settings key. */
  savedKey: string
  onSelect: (key: string) => void
  onOpenFolder: () => void
  onRemove: () => void
}

const IconAction = (props: {
  testId: string
  icon: string
  label: string
  hoverClass: string
  onClick: () => void
}) => (
  <Tooltip placement="top">
    <TooltipTrigger
      type="button"
      data-testid={props.testId}
      aria-label={props.label}
      class={`text-lightSlate-700 flex h-6 w-6 cursor-pointer items-center justify-center rounded-md border-0 bg-transparent p-0 transition-colors duration-100 ease-spring ${props.hoverClass}`}
      onClick={() => props.onClick()}
    >
      <div class={`${props.icon} h-4 w-4`} />
    </TooltipTrigger>
    <TooltipContent>{props.label}</TooltipContent>
  </Tooltip>
)

/** One line cut with an ellipsis; the full text shows in a tooltip, only when
 *  it is actually cut. */
const TruncatedLine = (props: { text: string; class: string }) => {
  let el: HTMLElement | undefined
  const [open, setOpen] = createSignal(false)
  return (
    <Tooltip
      placement="top"
      open={open()}
      onOpenChange={(next) =>
        setOpen(next && !!el && el.scrollWidth > el.clientWidth)
      }
    >
      <TooltipTrigger
        as="div"
        ref={(node) => (el = node)}
        class={`min-w-0 truncate ${props.class}`}
      >
        {props.text}
      </TooltipTrigger>
      <TooltipContent class="max-w-80 break-words">{props.text}</TooltipContent>
    </Tooltip>
  )
}

const InstalledThemeTile = (props: Props) => {
  const [t] = useTransContext()
  const m = () => props.model
  // A preview that doesn't load falls back to the drawn one.
  const [failedUrl, setFailedUrl] = createSignal<string | null>(null)
  const preview = () => {
    const url = m().previewUrl
    return url && url !== failedUrl() ? url : null
  }

  return (
    <div class="w-42">
      <div
        data-testid={`theme-tile-${m().key}`}
        class="w-42 relative rounded-md p-2 transition-transform duration-300 ease-spring"
        classList={{
          "hover:scale-105": m().selectable,
          "opacity-60": !!m().warning,
          "scale-105": m().active
        }}
        style={{ "background-color": m().shades[0] }}
        onClick={() => {
          if (!m().selectable) return
          props.onSelect(m().key)
        }}
        onMouseEnter={() => {
          if (!m().selectable) return
          applyThemeByName(m().key, { preview: true })
        }}
        onMouseLeave={() => {
          if (!m().selectable) return
          applyThemeByName(props.savedKey)
        }}
      >
        <div
          class="bg-darkSlate-500 absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full p-2 opacity-0 shadow-md transition-opacity duration-300 ease-spring"
          classList={{ "opacity-100": m().active }}
        >
          <div class="i-hugeicons:tick-02 h-6 w-6" />
        </div>

        <Show
          when={preview()}
          fallback={
            <ThemePreview
              shade1={m().shades[0]}
              shade2={m().shades[1]}
              shade3={m().shades[2]}
            />
          }
        >
          {(src) => (
            <img
              src={src()}
              alt=""
              class="aspect-[1512/830] block w-full rounded-sm object-cover"
              onError={() => setFailedUrl(src())}
            />
          )}
        </Show>
      </div>

      <div class="box-border flex w-full flex-col gap-0.5 px-2 py-4">
        <div class="flex min-w-0 items-center gap-1.5">
          <Show when={m().warning}>
            {(w) => (
              <Tooltip placement="top">
                <TooltipTrigger
                  as="div"
                  tabIndex={0}
                  role="img"
                  data-testid={`theme-warning-${props.id}`}
                  aria-label={t(w().reason)}
                  class="flex shrink-0 items-center"
                >
                  <div class="i-hugeicons:alert-diamond h-4 w-4 text-yellow-500" />
                </TooltipTrigger>
                <TooltipContent>
                  <div class="max-w-80 flex flex-col gap-1">
                    <span>{t(w().reason)}</span>
                    <Show when={w().detail}>
                      {(detail) => (
                        <span class="break-words font-mono text-xs">
                          {detail()}
                        </span>
                      )}
                    </Show>
                  </div>
                </TooltipContent>
              </Tooltip>
            )}
          </Show>
          <TruncatedLine text={m().name} class="text-lightSlate-700 flex-1" />
          <div class="flex shrink-0 items-center">
            <IconAction
              testId={`theme-open-folder-${props.id}`}
              icon="i-hugeicons:folder-open"
              label={t("settings:_trn_theme_open_folder")}
              hoverClass="hover:text-lightSlate-50"
              onClick={() => props.onOpenFolder()}
            />
            <Show when={m().removable}>
              <IconAction
                testId={`theme-remove-${props.id}`}
                icon="i-hugeicons:delete-02"
                label={t("settings:_trn_theme_remove")}
                hoverClass="hover:text-red-400"
                onClick={() => props.onRemove()}
              />
            </Show>
          </div>
        </div>
        <Show when={m().author}>
          {(author) => (
            <TruncatedLine
              text={author()}
              class="text-lightSlate-700 text-xs"
            />
          )}
        </Show>
      </div>
    </div>
  )
}

export default InstalledThemeTile
