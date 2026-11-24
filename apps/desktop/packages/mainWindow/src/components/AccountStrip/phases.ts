import { createSignal, onCleanup } from "solid-js"

/**
 * When the strip is in the page and when it is shown. Opening mounts it
 * hidden and shows it on the next frame, so its closed look is painted first
 * and the open look transitions from it. Closing hides it at once and keeps
 * it mounted until its closing animation (`closeMs`) is over; opening again
 * meanwhile keeps it.
 */
export function createStripPhases(opts: {
  closeMs: () => number
  nextFrame?: (cb: () => void) => void
}) {
  const nextFrame = opts.nextFrame ?? ((cb) => requestAnimationFrame(cb))
  const [mounted, setMounted] = createSignal(false)
  const [shown, setShown] = createSignal(false)
  let wanted = false
  let unmount: ReturnType<typeof setTimeout> | undefined

  const open = () => {
    wanted = true
    clearTimeout(unmount)
    setMounted(true)
    nextFrame(() => {
      if (wanted) setShown(true)
    })
  }

  const close = () => {
    wanted = false
    setShown(false)
    clearTimeout(unmount)
    const ms = opts.closeMs()
    if (ms <= 0) setMounted(false)
    else
      unmount = setTimeout(() => {
        if (!wanted) setMounted(false)
      }, ms)
  }

  onCleanup(() => clearTimeout(unmount))

  return { mounted, shown, open, close }
}
