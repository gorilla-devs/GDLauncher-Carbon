import { Rive } from "@rive-app/canvas"
import { createSignal, onCleanup, onMount } from "solid-js"
import "./riveRuntime"

// The intro holds the window back until it stops, so it has to stop whatever
// happens. Its engine and file load in well under a second and it plays for
// four; the library says nothing when its engine cannot be loaded.
const LOAD_LIMIT_MS = 4_000
const PLAY_LIMIT_MS = 15_000

interface Props {
  src: string
  onStop?: () => void
  width?: number
  height?: number
}

const RiveAppWrapper = (props: Props) => {
  let canvas: HTMLCanvasElement | undefined

  const [riveRef, setRiveRef] = createSignal<Rive | undefined>()
  let limit: ReturnType<typeof setTimeout> | undefined
  let stopped = false
  const stop = () => {
    clearTimeout(limit)
    if (stopped) return
    stopped = true
    props.onStop?.()
  }
  onMount(() => {
    limit = setTimeout(stop, LOAD_LIMIT_MS)
    if (canvas && props.src) {
      const r = new Rive({
        src: props.src,
        autoplay: true,
        canvas: canvas,
        stateMachines: ["State Machine 1"],
        onLoad: () => {
          r.resizeDrawingSurfaceToCanvas()
          setRiveRef(r)
          if (stopped) return
          clearTimeout(limit)
          limit = setTimeout(stop, PLAY_LIMIT_MS)
        },
        onLoadError: stop,
        onStateChange: (state) => {
          // there is no way to dected the end of an animation in rive.app
          // to achive it, I added another state at the end of the animatio so I can detect it
          // sorry idk how to change the name of the state, so it's ""
          if ((state.data as string[])[0] === "") {
            stop()
          }
        }
      })
    }
  })

  onCleanup(() => {
    clearTimeout(limit)
    stopped = true
    if (riveRef()) {
      riveRef()?.stopRendering()
      riveRef()?.cleanup()
    }
  })
  return (
    <canvas
      ref={canvas}
      width={props.width || 600}
      height={props.height || 600}
      style={{
        width: `${props.width || 600}px`,
        height: `${props.height || 600}px`
      }}
    />
  )
}

export default RiveAppWrapper
