import {
  Index,
  Show,
  createEffect,
  createMemo,
  onCleanup,
  untrack
} from "solid-js"
import { Portal } from "solid-js/web"
import { activeTheme } from "./active"
import {
  backgroundModel,
  retryVideoOnce,
  videoAction,
  warnOnLoadError,
  type BackgroundModel,
  type VideoAction
} from "./background"
import { motion } from "./motion"

// A blurred media element is sized explicitly, so it must not also be w-full h-full.
const sizeClass = (style: Record<string, string>) =>
  style.width ? "absolute" : "absolute w-full h-full"

const VideoLayer = (props: { model: () => BackgroundModel }) => {
  let el: HTMLVideoElement | undefined
  let prev: BackgroundModel | null = null

  const apply = (action: VideoAction) => {
    if (!el) return
    switch (action) {
      case "play":
        el.play().catch((e: unknown) => {
          if ((e as { name?: string })?.name !== "AbortError") {
            console.warn("Theme background video could not play", e)
          }
        })
        break
      case "pause":
        el.pause()
        break
      case "rewind":
        el.pause()
        el.currentTime = 0
        break
      case "release":
        el.pause()
        el.removeAttribute("src")
        el.load()
        break
    }
  }

  // load() leaves the element paused: put the playback state back. The retry
  // waits for run, so no timer runs while motion is paused or off.
  const retry = retryVideoOnce(untrack(motion), {
    onReload: () => apply(videoAction(null, props.model()))
  })
  createEffect(() => retry.setMotion(motion()))

  createEffect(() => {
    const next = props.model()
    apply(videoAction(prev, next))
    prev = next
  })

  onCleanup(() => {
    retry.dispose()
    apply(videoAction(prev, null))
    prev = null
    el = undefined
  })

  return (
    <video
      ref={el}
      class={sizeClass(props.model().media.style)}
      src={props.model().media.src}
      muted
      loop
      playsinline
      disablepictureinpicture
      style={props.model().media.style}
      onError={retry.onError}
      onLoadedData={retry.onLoaded}
    />
  )
}

const BackgroundLayer = () => {
  const model = createMemo(() =>
    backgroundModel(activeTheme()?.background ?? null, motion())
  )

  return (
    <Show when={model()}>
      {(m) => (
        <Portal mount={document.body}>
          <div
            id="gdl-background"
            data-gdl="background"
            aria-hidden="true"
            style={{
              position: "fixed",
              inset: "0",
              "z-index": "-1",
              "pointer-events": "none",
              overflow: "hidden"
            }}
          >
            <Show
              when={m().media.kind === "video"}
              fallback={
                <img
                  class={sizeClass(m().media.style)}
                  src={m().media.src}
                  alt=""
                  draggable={false}
                  decoding="async"
                  style={m().media.style}
                  onError={warnOnLoadError("image")}
                />
              }
            >
              <VideoLayer model={m} />
            </Show>
            {/* Index keeps each motion <img> across re-applies, so animations don't restart. */}
            <Index each={m().motion}>
              {(layer) => (
                <img
                  class="absolute w-full h-full"
                  src={layer().src}
                  alt=""
                  draggable={false}
                  decoding="async"
                  style={layer().style}
                  onError={warnOnLoadError("motion layer")}
                />
              )}
            </Index>
            <Show when={m().dim > 0}>
              <div
                class="absolute inset-0"
                style={{ background: `rgb(0 0 0 / ${m().dim})` }}
              />
            </Show>
          </div>
        </Portal>
      )}
    </Show>
  )
}

export default BackgroundLayer
