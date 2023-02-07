import {
  For,
  createEffect,
  createSignal,
  mergeProps,
  onCleanup,
  onMount,
} from "solid-js";

interface Marks {
  [mark: number]: string;
}
export interface Props {
  max: number;
  min: number;
  steps?: number;
  snap?: boolean;
  marks: Marks;
  defaultValue: number;
  onChange?: (_val: number) => void;
}

function Slider(props: Props) {
  const minValue = () => props.min;
  const [currentValue, setCurrentValue] = createSignal<number>(minValue() || 0);
  const [startPosition, setStartPosition] = createSignal<number>(0);
  const [startValue, setStartValue] = createSignal<number>(0);
  const [dragging, setDragging] = createSignal(false);

  let handleRef: HTMLDivElement;
  let sliderRef: HTMLDivElement;

  const mergedProps = mergeProps({ steps: 1 }, props);

  const findClosestNumberAndIndex = (arr: string[], target: number) => {
    let closest = parseInt(arr[0], 10);
    let index = 0;
    let minDiff = Math.abs(target - closest);
    for (let i = 1; i < arr.length; i++) {
      let diff = Math.abs(target - parseInt(arr[i], 10));
      if (diff < minDiff) {
        closest = parseInt(arr[i], 10);
        index = i;
        minDiff = diff;
      }
    }
    return { number: closest, index: index };
  };

  const getSliderStart = () => {
    const rect = sliderRef.getBoundingClientRect();
    return rect.left;
  };

  const getSliderLength = () => {
    if (!sliderRef) {
      return 0;
    }

    return sliderRef.clientWidth;
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
    if (mergedProps.steps !== null) {
      const closestStep =
        Math.round((val - props.min) / mergedProps.steps) * mergedProps.steps +
        props.min;
      points.push(closestStep);
    }

    const diffs = points.map((point) => Math.abs(val - point));
    const closestPoint = points[diffs.indexOf(Math.min.apply(Math, diffs))];

    return mergedProps.steps !== null
      ? parseFloat(closestPoint.toFixed(getPrecision(mergedProps.steps)))
      : closestPoint;
  };

  const calcValueByPos = (position: number) => {
    const pixelOffset = position - getSliderStart();
    const nextValue = trimAlignValue(calcValue(pixelOffset));
    return nextValue;
  };

  const mousedown = (e: MouseEvent) => {
    e.preventDefault();

    const value = calcValueByPos(e.pageX);
    setDragging(true);
    setStartPosition(e.pageX);
    setStartValue(value);

    if (currentValue() !== value) {
      onChange(value);
    }
  };

  const onChange = (val: number) => {
    setCurrentValue(val);
    props?.onChange?.(val);
  };

  const mousemove = (e: MouseEvent) => {
    if (!dragging()) return;
    let diffPosition = e.pageX - startPosition();
    const diffValue =
      (diffPosition / getSliderLength()) * (props.max - props.min);

    const value = trimAlignValue(startValue() + diffValue);
    const oldValue = currentValue();
    if (value === oldValue) return;

    onChange(value);
  };

  const mouseup = () => {
    setDragging(false);
  };

  onMount(() => {
    handleRef.addEventListener("mousedown", mousedown);
    document.addEventListener("mousemove", mousemove);
    document.addEventListener("mouseup", mouseup);
  });

  onCleanup(() => {
    handleRef.removeEventListener("mousedown", mousedown);
    document.removeEventListener("mousemove", mousemove);
    document.removeEventListener("mouseup", mouseup);
  });

  const calcOffset = (value: number) => {
    const ratio = (value - props.min) / (props.max - props.min);
    return ratio * 100;
  };

  return (
    <div class="relative">
      <For each={Object.entries(props.marks)}>
        {([value, label]) => (
          <span
            class="flex flex-col -top-1"
            style={{
              position: "absolute",
              left: `${value}%`,
            }}
          >
            <div class="w-2 h-2 bg-primary rounded-full border-4 border-solid border-primary" />
            <p class="mt-2 mb-0 text-xs">{label}</p>
          </span>
        )}
      </For>
      <div
        ref={(el) => {
          handleRef = el;
        }}
        class="w-4 h-4 bg-shade-8 rounded-full border-4 border-solid border-primary -top-2 cursor-move"
        style={{
          position: "absolute",
          // left: `${props.defaultValue}%`,
          left: `${calcOffset(currentValue())}%`,
          transform: !props.snap ? `translateX(-50%)` : "",
        }}
      />
      <div
        ref={(el) => {
          sliderRef = el;
        }}
        class="w-full h-2 bg-primary rounded-full"
      />
    </div>
  );
}

export { Slider };
