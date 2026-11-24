import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  Show,
  untrack
} from "solid-js"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button } from "@gd/ui"
import { i18n, Trans } from "@gd/i18n"
import { getThemeKey } from "@gd/i18n/helpers"
import { rspc } from "@/utils/rspcClient"
import { DecorationSlot } from "@/themes/DecorationSlot"
import { activeTheme } from "@/themes/active"
import { getBuiltinSeasonArt, isBuiltinThemeId } from "@/themes/builtin"
import { motion } from "@/themes/motion"
import { browserLoopDeps, createEffectsLoop } from "@/themes/effects/loop"
import {
  createFlakeSprite,
  createSnow,
  resolveSnowOptions
} from "@/themes/effects/snow"
import { formatIsoDay, splitOnName } from "@/themes/season"

const ART_WIDTH = 470
const ART_HEIGHT = 178
const SNOW_DENSITY_BOOST = 1.4

const SeasonalTheme = (props: ModalProps) => {
  const modals = useModal()
  const themeId = (): string => props.data?.themeId ?? ""
  const endsOn = (): string => props.data?.endsOn ?? ""

  const builtinId = createMemo(() => {
    const id = themeId()
    return isBuiltinThemeId(id) ? id : null
  })
  const art = () => {
    const id = builtinId()
    return id ? getBuiltinSeasonArt(id) : null
  }
  const name = () => {
    const id = builtinId()
    return id ? i18n.t(getThemeKey(id)) : themeId()
  }
  const text = () => {
    const [before, after] = splitOnName(
      i18n.t("settings:_trn_season_dialog_text", {
        name: "{{name}}",
        until: formatIsoDay(endsOn(), i18n.language || "english"),
        interpolation: { escapeValue: false }
      })
    )
    return { before, after }
  }

  const ackMutation = rspc.createMutation(() => ({
    mutationKey: ["themes.ackSeasonNotice"]
  }))
  const switchBackMutation = rspc.createMutation(() => ({
    mutationKey: ["themes.seasonSwitchBack"]
  }))

  // Escape and the backdrop close without a button: acknowledge once.
  let handled = false
  onCleanup(() => {
    if (handled) return
    handled = true
    ackMutation.mutate(undefined)
  })

  const switchBack = () => {
    handled = true
    switchBackMutation.mutate({ id: themeId() })
    modals?.closeModal("seasonalTheme")
  }
  const keep = () => {
    handled = true
    ackMutation.mutate(undefined)
    modals?.closeModal("seasonalTheme")
  }

  // The snow over the art: the active theme's own snow options, a bit denser
  // because the box is small. run animates, pause keeps the last frame.
  const [canvas, setCanvas] = createSignal<HTMLCanvasElement>()
  const loop = createEffectsLoop(browserLoopDeps)
  onCleanup(() => loop.setRunning(false))

  const isOff = createMemo(() => motion() === "off")

  createEffect(() => {
    const el = canvas()
    if (!el || isOff()) return
    const ctx = el.getContext("2d")
    if (!ctx) return
    const cfg = untrack(() =>
      activeTheme()?.effects.find((e) => e.type === "snow")
    )
    const base = resolveSnowOptions(cfg?.type === "snow" ? cfg : {})
    const snow = createSnow(
      { ...base, density: base.density * SNOW_DENSITY_BOOST },
      ctx,
      {
        random: Math.random,
        sprite: createFlakeSprite
      }
    )
    const remove = loop.add(snow)
    snow.resize(ART_WIDTH, ART_HEIGHT)
    onCleanup(() => {
      loop.setRunning(false)
      remove()
    })
  })

  createEffect(() => loop.setRunning(!!canvas() && motion() === "run"))

  return (
    <ModalLayout
      noHeader
      noPadding
      overflowHiddenDisabled
      scrollable="overflow-visible"
      width="w-[470px]"
      title={props.title}
    >
      {/* -mt-5 offsets ModalLayout's internal pt-5 */}
      <div data-gdl="season-dialog" class="relative isolate -mt-5">
        <DecorationSlot anchor="season-dialog" />
        <div
          data-gdl="season-dialog.art"
          class="relative overflow-hidden rounded-t-[20px]"
          style={{ height: `${ART_HEIGHT}px` }}
        >
          <Show when={art()}>
            {(src) => (
              <img
                src={src()}
                alt=""
                class="absolute inset-0 h-full w-full object-cover object-bottom"
              />
            )}
          </Show>
          <Show when={motion() !== "off"}>
            <canvas
              ref={setCanvas}
              aria-hidden="true"
              class="pointer-events-none absolute inset-0 h-full w-full"
            />
          </Show>
        </div>
        <div class="px-7 pb-6 pt-[22px]">
          <h3 class="m-0 mb-1.5 text-[21px] font-bold text-lightSlate-50">
            <Trans key="settings:_trn_season_dialog_title" />
          </h3>
          <p class="m-0 text-[14.5px] leading-normal text-lightSlate-400">
            {text().before}
            <b
              data-gdl="season-dialog.name"
              class="font-medium text-lightSlate-50"
            >
              {name()}
            </b>
            {text().after}
          </p>
          <div class="mt-[22px] flex justify-end gap-2.5">
            <Button
              type="secondary"
              size="large"
              data-testid="season-switch-back"
              onClick={switchBack}
            >
              <Trans key="settings:_trn_season_switch_back" />
            </Button>
            <Button
              type="primary"
              size="large"
              data-testid="season-keep"
              onClick={keep}
            >
              <Trans key="settings:_trn_season_keep" />
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  )
}

export default SeasonalTheme
