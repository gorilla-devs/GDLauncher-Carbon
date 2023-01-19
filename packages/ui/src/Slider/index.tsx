import { onMount } from "solid-js";
import Style from "./Slider.module.scss";

export interface Props {
  min: number;
  max: number;
  value: number;
  // eslint-disable-next-line no-unused-vars
  onChange?: (val: string | undefined) => void;
}

function Slider(props: Props) {
  let ref: HTMLInputElement | undefined;

  onMount(() => {
    if (ref) {
      ref.style.setProperty("--value", ref.value);
      ref.style.setProperty("--min", ref.min == "" ? "0" : ref.min);
      ref.style.setProperty("--max", ref.max == "" ? "100" : ref.max);
      ref.addEventListener("input", () => {
        props?.onChange?.(ref?.value);
        ref?.style.setProperty("--value", ref.value);
      });
    }
  });
  return (
    <input
      ref={ref}
      class={`${Style.slider} ${Style.sliderProgress} w-full`}
      style={{
        "--value": props.value,
        "--min": props.min || 0,
        "--max": props.max || 100,
      }}
      type="range"
      value="0"
    />
  );
}

export { Slider };
