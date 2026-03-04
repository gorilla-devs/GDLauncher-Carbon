import {
  For,
  Match,
  Switch,
  createSignal,
  onCleanup,
  onMount,
  JSX,
  Show,
  mergeProps
} from "solid-js"

type Marks = Record<number, string | JSX.Element | { label: JSX.Element }>
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

  const mergedProps = mergeProps({ noLabels: false, noTooltip: false }, props)

  let sliderRef: HTMLDivElement

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

    return props.steps !== null && props.steps !== undefined && closestPoint
      ? parseFloat(closestPoint.toFixed(getPrecision(props.steps)))
      : closestPoint
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
        class="group relative flex items-center box-border"
        classList={{
          "h-10 w-full max-w-full": !props.vertical,
          "h-full w-10": props.vertical
        }}
      >
        <Show when={showTooptip() && !mergedProps.noTooltip}>
          <div
            class="absolute bg-darkSlate-900 rounded-lg px-2 py-1"
            style={{
              position: "absolute",
              left: `${calcOffset(currentValue())}%`,
              transform: "translate(-50%, -40px)"
            }}
          >
            <div class="z-10 relative">{currentValue()}</div>
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
                    class="flex flex-col mb-0 text-xs text-darkGray-300 font-semibold"
                    classList={{
                      "-ml-2 mt-2 max-w-25": !props.vertical,
                      "-mt-2 mr-2": props.vertical
                    }}
                    style={{
                      position: "absolute",
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
            class="rounded-full border-0 bg-lightSlate-50/15 shadow-md cursor-grab z-20 transition-[color,box-shadow,transform,opacity,background-color] duration-100 ease-out group-hover:bg-lightSlate-50/80"
            style={{
              position: "absolute",
              ...(props.vertical
                ? {
                    top: `${calcOffset(currentValue())}%`,
                    transform: "translateY(-50%)"
                  }
                : {
                    left: `${calcOffset(currentValue())}%`,
                    transform: "translateX(-50%)"
                  })
            }}
            classList={{
              "w-1.5 h-5": !props.vertical,
              "h-1.5 w-5": props.vertical,
              "bg-lightSlate-50/80 scale-110 cursor-grabbing": showTooptip(),
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
            class="bg-darkSlate-600 rounded-full relative overflow-hidden"
            classList={{
              "w-full h-4": !props.vertical,
              "h-full w-4": props.vertical
            }}
          >
            <div
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
