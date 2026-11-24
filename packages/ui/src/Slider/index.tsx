import {
  For,
  Match,
  Switch,
  createSignal,
  onCleanup,
  onMount,
  JSX,
  Show,
  createEffect,
  mergeProps
} from "solid-js"

type Marks = Record<number, string | JSX.Element | { label: JSX.Element }>

/** A label's box along the slider, in px. */
export interface LabelBox {
  start: number
  end: number
}

/**
 * Which labels to hide so none overlaps the one shown before it: walking from
 * the start, a label is hidden when it begins less than `gap` px after the
 * last label kept. The first label is always kept. ("1 GB" and "2 GB" sit a
 * few px apart on a machine with a lot of memory, and read "1 GB2 GB".)
 */
export function overlappingLabels(boxes: LabelBox[], gap = 4): boolean[] {
  let lastEnd = -Infinity
  return boxes.map((box) => {
    if (box.end <= box.start) return false
    if (box.start < lastEnd + gap) return true
    lastEnd = box.end
    return false
  })
}
interface Props {
  max: number
  min: number
  steps?: number | null
  marks?: Marks
  value?: number
  noLabels?: boolean
  noTooltip?: boolean
  onChange?: (_val: number) => void
  OnRelease?: (_val: number) => void
  vertical?: boolean
  tooltipFormat?: (_val: number) => string | JSX.Element
}

function Slider(props: Props) {
  const defaultValue = () => props.value
  const min = () => props.min
  const [currentValue, setCurrentValue] = createSignal<number>(
    // eslint-disable-next-line solid/reactivity
    defaultValue() || min()
  )
  const [startPosition, setStartPosition] = createSignal<number>(0)
  const [startValue, setStartValue] = createSignal<number>(0)
  const [dragging, setDragging] = createSignal(false)
  const [showTooptip, setShowTooltip] = createSignal(false)
  const [handleRef, setHandleRef] = createSignal<HTMLDivElement | undefined>(
    undefined
  )

  createEffect(() => {
    const v = props.value
    if (v !== undefined) {
      setCurrentValue(v)
    }
  })

  const mergedProps = mergeProps({ noLabels: false, noTooltip: false }, props)

  let sliderRef: HTMLDivElement
  let rootRef: HTMLDivElement | undefined
  const labelRefs: HTMLElement[] = []
  const [hiddenLabels, setHiddenLabels] = createSignal<boolean[]>([])

  // Hides labels that would run into the one before them. Measured after
  // layout, and again whenever the slider's size or its marks change.
  const measureLabels = () => {
    const count = Object.keys(props.marks ?? {}).length
    const boxes = labelRefs.slice(0, count).map((el) => {
      if (!el?.isConnected) return { start: 0, end: 0 }
      const r = el.getBoundingClientRect()
      return props.vertical
        ? { start: r.top, end: r.bottom }
        : { start: r.left, end: r.right }
    })
    const next = overlappingLabels(boxes)
    const prev = hiddenLabels()
    if (next.length !== prev.length || next.some((h, i) => h !== prev[i])) {
      setHiddenLabels(next)
    }
  }

  let labelObserver: ResizeObserver | undefined
  const observeLabels = () => {
    const count = Object.keys(props.marks ?? {}).length
    for (const el of labelRefs.slice(0, count)) {
      if (el?.isConnected) labelObserver?.observe(el)
    }
  }

  const getSliderStart = () => {
    const rect = sliderRef.getBoundingClientRect()
    return props.vertical ? rect.top : rect.left
  }

  const getSliderLength = () => {
    if (!sliderRef) {
      return 0
    }

    return props.vertical ? sliderRef.clientHeight : sliderRef.clientWidth
  }

  const calcValue = (offset: number) => {
    const ratio = Math.abs(offset / getSliderLength())
    const value = ratio * (props.max - props.min) + props.min
    return value
  }

  const getPrecision = (step: number) => {
    const stepString = step.toString()
    let precision = 0
    if (stepString.includes(".")) {
      precision = stepString.length - stepString.indexOf(".") - 1
    }
    return precision
  }

  const trimAlignValue = (v: number) => {
    let val = v
    if (val <= props.min) {
      val = props.min
    }
    if (val >= props.max) {
      val = props.max
    }

    const points = props.marks ? Object.keys(props.marks).map(parseFloat) : []
    if (props.steps !== null && props.steps !== undefined) {
      const closestStep =
        Math.round((val - props.min) / props.steps) * props.steps + props.min
      points.push(closestStep)
    }

    if (points.length === 0) {
      return val
    }

    const diffs = points.map((point) => Math.abs(val - point))
    const closestPoint = points[diffs.indexOf(Math.min(...diffs))]

    const rounded =
      props.steps !== null && props.steps !== undefined && closestPoint
        ? parseFloat(closestPoint.toFixed(getPrecision(props.steps)))
        : closestPoint
    return Math.min(Math.max(rounded, props.min), props.max)
  }

  const calcValueByPos = (position: number) => {
    const pixelOffset = position - getSliderStart()
    const nextValue = trimAlignValue(calcValue(pixelOffset))
    return nextValue
  }

  const mousedown = (e: MouseEvent) => {
    e.preventDefault()

    const position = props.vertical ? e.pageY : e.pageX // Use pageY for vertical slider, pageX for horizontal
    const value = calcValueByPos(position)
    setDragging(true)
    setStartPosition(position)
    setStartValue(value)

    if (currentValue() !== value) {
      setCurrentValue(value)
    }

    // Add document listeners when dragging starts
    document.addEventListener("mousemove", mousemove)
    document.addEventListener("mouseup", mouseup)
  }

  const mousemove = (e: MouseEvent) => {
    if (!dragging()) return
    setShowTooltip(true)

    const position = props.vertical ? e.pageY : e.pageX
    const diffPosition = position - startPosition()

    const diffValue =
      (diffPosition / getSliderLength()) * (props.max - props.min)

    const value = trimAlignValue(startValue() + diffValue)
    const oldValue = currentValue()
    if (value === oldValue) return
    setCurrentValue(value)
  }

  const mouseup = () => {
    setShowTooltip(false)
    setDragging(false)
    props?.onChange?.(currentValue())
    props?.OnRelease?.(currentValue())

    // Remove document listeners when dragging ends
    document.removeEventListener("mousemove", mousemove)
    document.removeEventListener("mouseup", mouseup)
  }

  const trackMousedown = (e: MouseEvent) => {
    // Don't react if the click came from the handle itself.
    if (e.target === handleRef()) {
      return
    }

    // Delegate to the same mousedown handler so dragging starts from the track
    mousedown(e)
  }

  onMount(() => {
    const handle = handleRef()
    if (handle) {
      handle.addEventListener("mousedown", mousedown)
    }
    if (sliderRef) {
      sliderRef.addEventListener("mousedown", trackMousedown)
    }
    measureLabels()
    // Web fonts change the labels' widths once they load.
    void document.fonts?.ready.then(measureLabels)
    if (rootRef && typeof ResizeObserver !== "undefined") {
      // The slider's own size, and each label's (a theme's font or a new
      // value), change where the labels fall.
      labelObserver = new ResizeObserver(() => measureLabels())
      labelObserver.observe(rootRef)
      observeLabels()
      onCleanup(() => labelObserver?.disconnect())
    }
  })

  createEffect(() => {
    // Re-measure when the marks change (a new total memory, say).
    void Object.keys(props.marks ?? {}).length
    queueMicrotask(() => {
      observeLabels()
      measureLabels()
    })
  })

  onCleanup(() => {
    const handle = handleRef()
    if (handle) {
      handle.removeEventListener("mousedown", mousedown)
    }
    if (sliderRef) {
      sliderRef.removeEventListener("mousedown", trackMousedown)
    }
    // Clean up document listeners if still attached (e.g., component unmounts while dragging)
    if (dragging()) {
      document.removeEventListener("mousemove", mousemove)
      document.removeEventListener("mouseup", mouseup)
    }
  })

  const calcOffset = (value: number) => {
    const ratio = (value - props.min) / (props.max - props.min)
    return ratio * 100
  }

  return (
    <>
      <div
        ref={rootRef}
        data-gdl="slider"
        class="group relative flex items-center box-border"
        classList={{
          "h-10 w-full max-w-full": !props.vertical,
          "h-full w-10": props.vertical
        }}
      >
        <Show when={showTooptip() && !mergedProps.noTooltip}>
          <div
            data-gdl="slider.tooltip"
            class="absolute bg-darkSlate-900 rounded-lg px-2 py-1"
            style={{
              position: "absolute",
              left: `${calcOffset(currentValue())}%`,
              transform: "translate(-50%, -40px)"
            }}
          >
            <div class="z-10 relative whitespace-nowrap">
              {props.tooltipFormat
                ? props.tooltipFormat(currentValue())
                : currentValue()}
            </div>
            <div class="z-1 absolute left-1/2 -translate-x-1/2 -bottom-1 w-3 h-3 rotate-45 bg-darkSlate-900" />
          </div>
        </Show>
        <div
          class="relative"
          classList={{
            "w-full": !props.vertical,
            "h-full": props.vertical
          }}
        >
          <Show when={props.marks}>
            <For each={Object.entries(props.marks!)}>
              {([value, label], i) => (
                <>
                  <div
                    data-gdl="slider.mark"
                    class="absolute z-10 bg-lightSlate-50/25"
                    style={{
                      ...(props.vertical
                        ? {
                            left: "50%",
                            top: `${calcOffset(parseInt(value, 10))}%`,
                            width: "1px",
                            height: "8px",
                            transform: "translate(-50%, -50%)"
                          }
                        : {
                            top: "50%",
                            left: `${calcOffset(parseInt(value, 10))}%`,
                            width: "1px",
                            height: "8px",
                            transform: "translate(-50%, -50%)"
                          })
                    }}
                  />
                  <p
                    ref={(el) => {
                      labelRefs[i()] = el
                    }}
                    data-gdl="slider.label"
                    class="flex flex-col mb-0 text-xs text-darkGray-300 font-semibold"
                    classList={{
                      "-ml-2 mt-2 max-w-25": !props.vertical,
                      "-mt-2 mr-2": props.vertical
                    }}
                    style={{
                      position: "absolute",
                      ...(hiddenLabels()[i()] ? { visibility: "hidden" } : {}),
                      ...(props.vertical
                        ? {
                            right: "10px",
                            top: `calc(${calcOffset(parseInt(value, 10))}% -  ${
                              i() === Object.entries(props.marks!).length - 1
                                ? "10px"
                                : "0px"
                            })`
                          }
                        : {
                            top: "10px",
                            left: `calc(${calcOffset(parseInt(value, 10))}% -  ${
                              i() === Object.entries(props.marks!).length - 1
                                ? "10px"
                                : "0px"
                            })`
                          })
                    }}
                  >
                    <Switch>
                      <Match
                        when={
                          typeof label === "string" && !mergedProps.noLabels
                        }
                      >
                        {label as string}
                      </Match>
                      <Match
                        when={
                          typeof label === "object" && !mergedProps.noLabels
                        }
                      >
                        {(label as { label: string }).label}
                      </Match>
                    </Switch>
                  </p>
                </>
              )}
            </For>
          </Show>
          <div
            ref={setHandleRef}
            data-gdl="slider.thumb"
            class="rounded-full border-0 shadow-md cursor-grab z-20 transition-[color,box-shadow,transform,opacity,background-color] ease-out group-hover:bg-lightSlate-50/80"
            style={{
              position: "absolute",
              ...(props.vertical
                ? { top: `${calcOffset(currentValue())}%` }
                : { left: `${calcOffset(currentValue())}%` })
            }}
            classList={{
              "w-1.5 h-5 -translate-x-1/2": !props.vertical,
              "h-1.5 w-5 -translate-y-1/2": props.vertical,
              "bg-lightSlate-50/80 scale-110": showTooptip() && !dragging(),
              "bg-lightSlate-50 scale-90 shadow-lg shadow-primary-500/40 cursor-grabbing":
                dragging(),
              "duration-200": !dragging(),
              "duration-75": dragging(),
              "-top-0.5": !props.vertical,
              "-left-0.5": props.vertical
            }}
            onMouseOver={() => {
              setShowTooltip(true)
            }}
            onMouseOut={() => {
              setShowTooltip(false)
            }}
          />
          <div
            ref={(el) => {
              sliderRef = el
            }}
            class="absolute z-10 cursor-pointer"
            classList={{
              "top-1/2 left-0 right-0 -translate-y-1/2 w-full h-4":
                !props.vertical,
              "top-0 bottom-0 left-1/2 -translate-x-1/2 h-full w-4":
                props.vertical
            }}
          />
          <div
            data-gdl="slider.track"
            class="bg-darkSlate-600 rounded-full relative overflow-hidden"
            classList={{
              "w-full h-4": !props.vertical,
              "h-full w-4": props.vertical
            }}
          >
            <div
              data-gdl="slider.fill"
              class="absolute inset-0 bg-primary-500"
              style={{
                ...(props.vertical
                  ? {
                      height: `${calcOffset(currentValue())}%`
                    }
                  : {
                      width: `${calcOffset(currentValue())}%`
                    })
              }}
            />
          </div>
        </div>
      </div>
    </>
  )
}

export { Slider }
