import {
  For,
  Match,
  Switch,
  createSignal,
  onCleanup,
  onMount,
  JSX,
  Show,
  mergeProps,
  createEffect,
} from "solid-js";

interface Marks {
  [mark: number]: string | JSX.Element | { label: JSX.Element };
}
interface Props {
  max: number;
  min: number;
  steps?: number | null;
  marks: Marks;
  value?: number;
  noLabels?: boolean;
  noTooltip?: boolean;
  onChange?: (_val: number) => void;
  OnRelease?: (_val: number) => void;
  vertical?: boolean;
}

function Slider(props: Props) {
  const defaultValue = () => props.value;
  const min = () => props.min;
  const [currentValue, setCurrentValue] = createSignal<number>(
    // eslint-disable-next-line solid/reactivity
    defaultValue() || min()
  );
  const [startPosition, setStartPosition] = createSignal<number>(0);
  const [startValue, setStartValue] = createSignal<number>(0);
  const [dragging, setDragging] = createSignal(false);
  const [showTooptip, setShowTooltip] = createSignal(false);
  const [handleRef, setHandleRef] = createSignal<HTMLDivElement | undefined>(
    undefined
  );

  const mergedProps = mergeProps({ noLabels: false, noTooltip: false }, props);

  let sliderRef: HTMLDivElement;

  const getSliderStart = () => {
    const rect = sliderRef.getBoundingClientRect();
    return props.vertical ? rect.top : rect.left;
  };

  const getSliderLength = () => {
    if (!sliderRef) {
      return 0;
    }

    return props.vertical ? sliderRef.clientHeight : sliderRef.clientWidth;
  };

  const calcValue = (offset: number) => {
    const ratio = Math.abs(offset / getSliderLength());
    const value = ratio * (props.max - props.min) + props.min;
    return value;
  };

  const getPrecision = (step: number) => {
    const stepString = step.toString();
    let precision = 0;
    if (stepString.indexOf(".") >= 0) {
      precision = stepString.length - stepString.indexOf(".") - 1;
    }
    return precision;
  };

  const trimAlignValue = (v: number) => {
    let val = v;
    if (val <= props.min) {
      val = props.min;
    }
    if (val >= props.max) {
      val = props.max;
    }

    const points = Object.keys(props.marks).map(parseFloat);
    if (props.steps !== null && props.steps !== undefined) {
      const closestStep =
        Math.round((val - props.min) / props.steps) * props.steps + props.min;
      points.push(closestStep);
    }

    const diffs = points.map((point) => Math.abs(val - point));
    const closestPoint = points[diffs.indexOf(Math.min.apply(Math, diffs))];

    return props.steps !== null && props.steps !== undefined && closestPoint
      ? parseFloat(closestPoint.toFixed(getPrecision(props.steps)))
      : closestPoint;
  };

  const calcValueByPos = (position: number) => {
    const pixelOffset = position - getSliderStart();
    const nextValue = trimAlignValue(calcValue(pixelOffset));
    return nextValue;
  };

  const mousedown = (e: MouseEvent) => {
    e.preventDefault();

    const position = props.vertical ? e.pageY : e.pageX; // Use pageY for vertical slider, pageX for horizontal
    const value = calcValueByPos(position);
    setDragging(true);
    setStartPosition(position);
    setStartValue(value);

    if (currentValue() !== value) {
      onChange(value);
    }
  };

  createEffect(() => {
    if (props.value !== undefined) onChange(props.value);
  });

  const onChange = (val: number) => {
    setCurrentValue(val);
    props?.onChange?.(val);
  };

  const mousemove = (e: MouseEvent) => {
    if (!dragging()) return;
    setShowTooltip(true);

    let position = props.vertical ? e.pageY : e.pageX;
    let diffPosition = position - startPosition();

    const diffValue =
      (diffPosition / getSliderLength()) * (props.max - props.min);

    const value = trimAlignValue(startValue() + diffValue);
    const oldValue = currentValue();
    if (value === oldValue) return;
    onChange(value);
  };

  const mouseup = () => {
    setShowTooltip(false);
    setDragging(false);
    props?.OnRelease?.(currentValue());
  };

  const trackClick = (e: MouseEvent) => {
    e.preventDefault();

    // Don't react if the click came from the handle itself.
    if (e.target === handleRef()) {
      return;
    }

    const position = props.vertical ? e.clientY : e.clientX; // Use clientY for vertical slider, clientX for horizontal
    const value = calcValueByPos(position);
    onChange(value);
    mouseup();
  };

  onMount(() => {
    handleRef()?.addEventListener("mousedown", mousedown);
    sliderRef.addEventListener("click", trackClick);
    document.addEventListener("mousemove", mousemove);
    document.addEventListener("mouseup", mouseup);
  });

  onCleanup(() => {
    handleRef()?.removeEventListener("mousedown", mousedown);
    sliderRef.removeEventListener("click", trackClick);
    document.removeEventListener("mousemove", mousemove);
    document.removeEventListener("mouseup", mouseup);
  });

  const calcOffset = (value: number) => {
    const ratio = (value - props.min) / (props.max - props.min);
    return ratio * 100;
  };

  return (
    <>
      <div
        class="relative flex items-center box-border mb-4"
        classList={{
          "h-10 w-full max-w-full": !props.vertical,
          "h-full w-10": props.vertical,
        }}
      >
        <Show when={showTooptip() && !mergedProps.noTooltip}>
          <div
            class="absolute bg-darkSlate-900 rounded-lg px-2 py-1"
            style={{
              position: "absolute",
              left: `${calcOffset(currentValue())}%`,
              transform: "translate(-50%, -40px)",
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
            "h-full": props.vertical,
          }}
        >
          <For each={Object.entries(props.marks)}>
            {([value, label], i) => (
              <>
                <div
                  class="w-2 h-2 rounded-full border-4 border-solid"
                  style={{
                    position: "absolute",

                    ...(props.vertical
                      ? {
                          top: `${calcOffset(parseInt(value, 10))}%`,
                          "margin-top": -(16 / 2) + "px",
                        }
                      : {
                          left: `${calcOffset(parseInt(value, 10))}%`,
                          "margin-left": -(16 / 2) + "px",
                        }),
                  }}
                  classList={{
                    "bg-darkSlate-900 border-darkSlate-900":
                      calcOffset(parseInt(value, 10)) >=
                      calcOffset(currentValue()),
                    "bg-primary-500 border-primary-500":
                      calcOffset(parseInt(value, 10)) <=
                        calcOffset(currentValue()) && !showTooptip(),
                    "bg-accent border-accent":
                      calcOffset(parseInt(value, 10)) <=
                        calcOffset(currentValue()) && showTooptip(),
                    "-top-1": !props.vertical,
                    "-right-1": props.vertical,
                  }}
                />
                <p
                  class="flex flex-col mb-0 text-xs text-lightSlate-500"
                  classList={{
                    "-ml-2 mt-2 max-w-25": !props.vertical,
                    "-mt-2 mr-2": props.vertical,
                  }}
                  style={{
                    position: "absolute",
                    ...(props.vertical
                      ? {
                          right: "10px",
                          top: `calc(${calcOffset(parseInt(value, 10))}% -  ${
                            i() === Object.entries(props.marks).length - 1
                              ? "10px"
                              : "0px"
                          })`,
                        }
                      : {
                          top: "10px",
                          left: `calc(${calcOffset(parseInt(value, 10))}% -  ${
                            i() === Object.entries(props.marks).length - 1
                              ? "10px"
                              : "0px"
                          })`,
                        }),
                  }}
                >
                  <Switch>
                    <Match
                      when={typeof label === "string" && !mergedProps.noLabels}
                    >
                      {label}
                    </Match>
                    <Match
                      when={typeof label === "object" && !mergedProps.noLabels}
                    >
                      {label.label}
                    </Match>
                  </Switch>
                </p>
              </>
            )}
          </For>
          <div
            ref={setHandleRef}
            class="w-4 h-4 bg-darkSlate-800 rounded-full border-4 border-solid border-primary-500 cursor-pointer z-20"
            style={{
              position: "absolute",
              ...(props.vertical
                ? {
                    top: `${calcOffset(currentValue())}%`,
                    transform: "translateY(-50%)",
                  }
                : {
                    left: `${calcOffset(currentValue())}%`,
                    transform: "translateX(-50%)",
                  }),
            }}
            classList={{
              "after:content-[] after:rounded-full after:absolute after:top-1/2 after:left-1/2 after:-translate-1/2 hover:after:shadow-[0_0_0_6px_var(--accent)] after:w-4 after:h-4 after:transition-shadow after:bg-darkSlate-800 after:ease-in-out after:duration-100 after:absolute after:top-1/2 after:left-1/2 after:-translate-1/2 after:shadow-[0_0_0_6px_var(--accent)] after:w-4 after:h-4 after:transition-shadow after:bg-darkSlate-800 after:ease-in-out after:duration-100 after:z-0 z-10":
                showTooptip(),
              "-top-2": !props.vertical,
              "-left-2": props.vertical,
            }}
            onMouseOver={() => {
              setShowTooltip(true);
            }}
            onMouseOut={() => {
              setShowTooltip(false);
            }}
          />
          <div
            ref={(el) => {
              sliderRef = el;
            }}
            class="absolute z-10 cursor-pointer"
            classList={{
              "top-1/2 left-0 right-0 -translate-y-1/2 w-full h-2":
                !props.vertical,
              "top-0 bottom-0 left-1/2 -translate-x-1/2 h-full w-2":
                props.vertical,
            }}
          />
          <div
            class="rounded-full"
            classList={{
              "bg-accent": showTooptip(),
              "bg-primary-500": !showTooptip(),
              "h-2": !props.vertical,
              "w-2": props.vertical,
            }}
            style={{
              position: "absolute",
              ...(props.vertical
                ? {
                    height: `${calcOffset(currentValue())}%`,
                  }
                : {
                    width: `${calcOffset(currentValue())}%`,
                  }),
            }}
          />
          <div
            class="bg-darkSlate-900 rounded-full"
            classList={{
              "w-full h-2": !props.vertical,
              "h-full w-2": props.vertical,
            }}
          />
        </div>
      </div>
    </>
  );
}

export { Slider };
