import { For, JSX, Show, createSignal } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getTaskTranslationKey } from "@gd/i18n/helpers"
import { FESubtask, Translation } from "@gd/core_module/bindings"
import {
  Checkbox,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  PRESS_CLASSES_LIGHT
} from "@gd/ui"
import DefaultImg from "/assets/images/default-instance-img.png"
import SelectionBorder from "../Instance/SelectionBorder"

export interface BaseTileProps {
  // Identity
  name: string

  // Sizing
  size: 1 | 2 | 3 | 4 | 5

  // Image
  img?: string

  // States — normalized across instance/server
  isLoading: boolean // Actively downloading/installing with progress
  isWaiting: boolean // Queued/indeterminate (no progress yet)
  isRunning: boolean
  isBusy: boolean // Starting/stopping (server only, false for instance)
  isDeleting: boolean
  isInvalid?: boolean // Instance only

  // Error
  failError?: string
  onDismissError?: () => void

  // Progress
  percentage?: number
  subTasks?: FESubtask[]
  downloaded?: number
  totalDownload?: number

  // Selection
  isMultiSelected: boolean
  showCheckbox: boolean
  onToggleSelection?: () => void

  // Drag
  onDragStart?: (e: PointerEvent) => void
  isDragging: boolean
  isDragActive: boolean
  canDrag: boolean

  // Click
  onClick?: (e: MouseEvent) => void
  onHover?: () => void

  // View transitions
  shouldSetViewTransition: boolean
  viewTransitionPrefix: string // "instance-tile" or "server-tile"

  // Play button
  onPlay?: (e: MouseEvent) => void
  playButtonContent: JSX.Element

  // Glow border extra class (e.g., "instance-tile-new")
  glowExtraClass?: string

  // Render slots
  infoContent: JSX.Element
  additionalOverlays?: JSX.Element

  // Waiting/deleting spinner text
  waitingText?: JSX.Element
  deletingText?: JSX.Element

  // Context menu state (managed by parent ContextMenu)
  isMenuOpen?: boolean
}

const getTranslationArgs = (translation: Translation) => {
  if ("args" in translation) {
    return translation.args
  }
  return {}
}

const BaseTile = (props: BaseTileProps) => {
  const [t] = useTransContext()
  const [copiedError, setCopiedError] = createSignal(false)
  const [isHovering, setIsHovering] = createSignal(false)

  const isMenuOpen = () => props.isMenuOpen ?? false

  const isLoadingOrWaiting = () => props.isLoading || props.isWaiting

  // Progress bar computation
  const computedProgressWidth = () => {
    const sub = props.subTasks?.find(
      (s) => s.progress !== "opaque"
    )
    if (!sub) return 0
    if ("download" in sub.progress) {
      const d = sub.progress.download
      return d.total > 0 ? (d.downloaded / d.total) * 100 : 0
    }
    if ("item" in sub.progress) {
      const i = sub.progress.item
      return i.total > 0 ? (i.current / i.total) * 100 : 0
    }
    return 0
  }

  const computedProgressText = () => {
    const sub = props.subTasks?.find(
      (s) => s.progress !== "opaque"
    )
    if (!sub) return ""
    if ("download" in sub.progress) {
      const d = sub.progress.download
      const dlMB = (d.downloaded / 1_048_576).toFixed(1)
      const totMB = (d.total / 1_048_576).toFixed(1)
      return `${dlMB} / ${totMB} MB`
    }
    if ("item" in sub.progress) {
      const i = sub.progress.item
      return `${i.current} / ${i.total}`
    }
    return ""
  }

  return (
    <div
      class={`group isolate relative flex select-none flex-col items-center justify-center ${PRESS_CLASSES_LIGHT}`}
      classList={{
        "opacity-0": props.isDragging,
        "cursor-grab": props.canDrag
      }}
      onClick={(e) => {
        e.stopPropagation()
        if (isLoadingOrWaiting() || props.isDeleting || props.isInvalid) return
        props.onClick?.(e)
      }}
      onPointerDown={(e) => {
        if (e.button !== 0) return
        if (isMenuOpen()) {
          document.body.dispatchEvent(
            new PointerEvent("pointerdown", { bubbles: true })
          )
          return
        }
        if (
          !isLoadingOrWaiting() &&
          !props.isDeleting &&
          props.canDrag &&
          props.onDragStart
        ) {
          props.onDragStart(e)
        }
      }}
      onMouseEnter={() => {
        setIsHovering(true)
        props.onHover?.()
      }}
      onMouseLeave={() => setIsHovering(false)}
    >
      <SelectionBorder
        isSelected={props.isMultiSelected}
        size={props.size}
      />

      <Tooltip
        open={props.failError ? undefined : false}
        placement="top"
      >
        <TooltipTrigger>
          <div
            class="relative box-border overflow-hidden rounded-2xl p-[2px]"
            classList={{
              [props.glowExtraClass || ""]: !!props.glowExtraClass
            }}
          >
            {/* Running / loading / busy border glow */}
            <div
              class="absolute left-0 top-0 h-full w-full transition-[opacity,background] duration-300 ease-spring"
              classList={{
                "opacity-0 bg-transparent":
                  !isLoadingOrWaiting() &&
                  !props.isRunning &&
                  !props.isBusy,
                "opacity-100":
                  isLoadingOrWaiting() ||
                  props.isRunning ||
                  props.isBusy,
                "bg-green-400": props.isRunning,
                "bg-yellow-400": props.isBusy && !props.isRunning,
                "instance-tile-spinning": isLoadingOrWaiting()
              }}
            />

            {/* Image container */}
            <div
              class="relative overflow-hidden rounded-2xl"
              classList={{
                "h-120 w-120": props.size === 5,
                "h-84 w-84": props.size === 4,
                "h-60 w-60": props.size === 3,
                "h-46 w-46": props.size === 2,
                "h-24 w-24": props.size === 1
              }}
              style={
                props.shouldSetViewTransition
                  ? {
                      "view-transition-name": `${props.viewTransitionPrefix}-image-container`,
                      contain: "layout"
                    }
                  : {}
              }
            >
              {/* Background image */}
              <div
                class="bg-darkSlate-800 relative flex h-full w-full items-center justify-center overflow-hidden rounded-2xl bg-cover bg-center transition-all duration-300 ease-spring"
                classList={{
                  grayscale: isLoadingOrWaiting(),
                  "group-hover:scale-110 group-hover:blur-[2px]":
                    !isLoadingOrWaiting() && !props.isDragActive,
                  "scale-110 blur-[2px]":
                    isMenuOpen() &&
                    !isLoadingOrWaiting() &&
                    !props.isDragActive
                }}
                style={{
                  "background-image": props.img
                    ? `url("${props.img}")`
                    : `url("${DefaultImg}")`,
                  "will-change": "transform, filter",
                  contain: "layout style",
                  ...(props.shouldSetViewTransition
                    ? {
                        "view-transition-name": `${props.viewTransitionPrefix}-image`
                      }
                    : {})
                }}
              />

              {/* Hover dark overlay */}
              <div
                class="z-1 absolute inset-0 rounded-2xl bg-black/0 transition-all duration-300 ease-spring"
                classList={{
                  "!bg-black/0": isLoadingOrWaiting(),
                  "group-hover:bg-black/30": !props.isDragActive,
                  "bg-black/30":
                    isMenuOpen() && !props.isDragActive
                }}
              />

              {/* Selection checkbox */}
              <Show
                when={
                  props.showCheckbox &&
                  props.onToggleSelection &&
                  !isLoadingOrWaiting()
                }
              >
                <div
                  class="z-10 absolute left-2 top-2 transition-all duration-200 ease-spring"
                  classList={{
                    "translate-x-0 opacity-100":
                      props.isMultiSelected ||
                      (isHovering() && !props.isDragActive),
                    "-translate-x-3 opacity-0":
                      !props.isMultiSelected &&
                      (!isHovering() || props.isDragActive)
                  }}
                  onPointerDown={(e) => {
                    e.stopPropagation()
                    if (isMenuOpen()) {
                      document.body.dispatchEvent(
                        new PointerEvent("pointerdown", { bubbles: true })
                      )
                    }
                  }}
                  onClick={(e) => {
                    e.stopPropagation()
                    e.preventDefault()
                    props.onToggleSelection?.()
                  }}
                >
                  <Checkbox
                    checked={props.isMultiSelected}
                    hover={false}
                  />
                </div>
              </Show>

              {/* Invalid state overlay (instance only) */}
              <Show when={props.isInvalid}>
                <h2 class="z-2 absolute left-0 top-0 text-center text-sm">
                  <Trans key="instances:_trn_error_invalid" />
                </h2>
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-black from-30% opacity-50" />
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t from-black opacity-50" />
                <div class="i-hugeicons:alert-01 z-1 absolute right-1 top-1 text-2xl text-yellow-500 shrink-0" />
              </Show>

              {/* Error state overlay */}
              <Show when={props.failError}>
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-black from-30% opacity-60" />
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t from-black opacity-60" />
                <div class="i-hugeicons:alert-01 z-1 absolute bottom-20 left-0 right-0 top-0 m-auto text-4xl text-red-500 shrink-0" />
                <div class="z-3 absolute left-1/2 top-1/2 mt-5 w-full -translate-x-1/2 -translate-y-1/2 text-center">
                  <div class="text-3xl font-bold">
                    <Trans key="general:_trn_error" />
                  </div>
                  <div class="text-sm">
                    (<Trans key="general:_trn_hover_for_details" />)
                  </div>
                  <Show when={props.onDismissError}>
                    <button
                      class="text-lightSlate-400 hover:text-lightSlate-200 mt-2 text-xs underline"
                      onClick={(e) => {
                        e.stopPropagation()
                        props.onDismissError!()
                      }}
                    >
                      <Trans key="general:_trn_dismiss" />
                    </button>
                  </Show>
                </div>
              </Show>

              {/* Loading dark overlays (backdrop-blur + gradients) */}
              <Show
                when={
                  isLoadingOrWaiting() || props.isDeleting
                }
              >
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 rounded-2xl backdrop-blur-sm" />
                <div class="from-darkSlate-900 z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-30% opacity-50" />
                <div class="from-darkSlate-900 z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t opacity-50" />
              </Show>

              {/* Progress percentage display */}
              <Show
                when={
                  props.isLoading &&
                  props.percentage !== undefined &&
                  props.percentage !== null
                }
              >
                <div
                  class="z-3 animate-enterWithOpacityChange absolute left-0 top-0 box-border flex h-full w-full flex-col items-center justify-center gap-2 p-2 opacity-0"
                  style={
                    props.shouldSetViewTransition
                      ? {
                          "view-transition-name": `${props.viewTransitionPrefix}-progress-text`
                        }
                      : {}
                  }
                >
                  <h3 class="m-0 text-center text-3xl">
                    {Math.round(props.percentage!)}%
                  </h3>
                  <div class="text-lightSlate-300 h-10">
                    <For each={props.subTasks}>
                      {(subTask) => (
                        <div
                          class="text-center"
                          classList={{
                            "text-xs":
                              props.subTasks &&
                              props.subTasks.length > 1,
                            "text-md":
                              props.subTasks?.length === 1
                          }}
                        >
                          {t(
                            getTaskTranslationKey(
                              subTask.name.translation
                            ),
                            getTranslationArgs(subTask.name)
                          )}
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>

              {/* Waiting spinner (queue / indeterminate installing) */}
              <Show when={props.isWaiting && !props.isLoading}>
                <div class="z-3 absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center gap-2">
                  <Spinner />
                  <span class="text-sm font-bold text-white">
                    {props.waitingText}
                  </span>
                </div>
              </Show>

              {/* Deleting spinner */}
              <Show when={props.isDeleting}>
                <div class="z-3 absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center gap-2">
                  <Spinner />
                  <span class="font-bold text-white">
                    {props.deletingText || (
                      <Trans key="instances:_trn_isDeleting" />
                    )}
                  </span>
                </div>
              </Show>

              {/* Additional overlays (modpack icon, NEW badge, etc.) */}
              {props.additionalOverlays}

              {/* Play/Stop button */}
              <Show
                when={
                  !isLoadingOrWaiting() &&
                  !props.isDeleting &&
                  !props.isInvalid &&
                  !props.failError
                }
              >
                <div
                  class="z-5 absolute right-3 top-3 h-10 items-center justify-center gap-2 rounded-xl px-4 transition-all duration-200 ease-spring translate-x-3 opacity-0"
                  classList={{
                    "flex bg-primary-500 hover:bg-primary-400":
                      !props.isRunning && !props.isBusy,
                    hidden: props.isRunning || props.isBusy,
                    "flex bg-red-500 translate-x-0 opacity-100":
                      props.isRunning,
                    "flex bg-yellow-500 translate-x-0 opacity-100":
                      props.isBusy && !props.isRunning,
                    "group-hover:flex group-hover:translate-x-0 group-hover:opacity-100":
                      !props.isRunning &&
                      !props.isBusy &&
                      !props.isDragActive,
                    "!flex !translate-x-0 !opacity-100":
                      isMenuOpen() &&
                      !props.isRunning &&
                      !props.isBusy
                  }}
                  style={
                    props.shouldSetViewTransition
                      ? {
                          "view-transition-name": `${props.viewTransitionPrefix}-play-button`,
                          contain: "layout"
                        }
                      : {}
                  }
                  onClick={(e) => {
                    e.stopPropagation()
                    props.onPlay?.(e)
                  }}
                >
                  {props.playButtonContent}
                </div>
              </Show>

              {/* Info overlay */}
              <div
                class="z-4 absolute bottom-0 left-0 right-0 flex flex-col gap-1 p-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent rounded-b-2xl transition-opacity duration-300"
                classList={{
                  "opacity-0":
                    isLoadingOrWaiting() || props.isDeleting
                }}
              >
                {props.infoContent}
              </div>

              {/* Subtask progress bar */}
              <Show
                when={
                  props.isLoading &&
                  props.subTasks?.length &&
                  props.subTasks.find(
                    (s) => s.progress !== "opaque"
                  )
                }
              >
                <div class="z-5 animate-enterWithOpacityChange absolute bottom-0 left-0 right-0 flex items-center gap-2 p-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent rounded-b-2xl overflow-hidden opacity-0">
                  <div class="relative min-w-0 flex-1 overflow-hidden rounded-full h-2">
                    <div class="bg-darkSlate-500/50 absolute inset-0 rounded-full" />
                    <div
                      class="bg-primary-500 absolute left-0 top-0 h-full rounded-full transition-all duration-300 ease-linear"
                      style={{
                        width: `${computedProgressWidth()}%`
                      }}
                    />
                  </div>
                  <Show when={props.size >= 2}>
                    <span
                      class="text-lightSlate-200 shrink-0 whitespace-nowrap font-medium"
                      classList={{
                        "text-sm": props.size >= 3,
                        "text-xs": props.size === 2
                      }}
                    >
                      {computedProgressText()}
                    </span>
                  </Show>
                </div>
              </Show>
            </div>
          </div>
        </TooltipTrigger>

        {/* Error tooltip */}
        <TooltipContent class="!p-0 !text-sm max-w-80 border border-solid border-darkSlate-500 shadow-lg shadow-darkSlate-900/50">
          <div class="flex flex-col">
            <div class="flex items-center justify-between gap-4 px-4 pt-3 pb-2">
              <div class="flex items-center gap-2 text-red-400 font-semibold">
                <div class="i-hugeicons:alert-01 h-4 w-4 shrink-0" />
                <Trans key="general:_trn_error" />
              </div>
              <div
                class={`${copiedError() ? "i-hugeicons:tick-double-02" : "i-hugeicons:copy-01"} h-4 w-4 shrink-0 cursor-pointer transition-colors duration-150`}
                classList={{
                  "text-lightSlate-500 hover:text-lightSlate-200":
                    !copiedError(),
                  "text-green-400": copiedError()
                }}
                onClick={(e) => {
                  e.stopPropagation()
                  navigator.clipboard.writeText(props.failError!)
                  setCopiedError(true)
                  setTimeout(() => setCopiedError(false), 2000)
                }}
              />
            </div>
            <div class="h-px bg-darkSlate-600 mx-3" />
            <div class="px-4 py-3 text-lightSlate-300 break-words leading-relaxed max-h-40 overflow-y-auto">
              {props.failError}
            </div>
          </div>
        </TooltipContent>
      </Tooltip>
    </div>
  )
}

// Export the menu open signal setter so consumers can sync context menu state
export { BaseTile }
export type { BaseTileProps }
