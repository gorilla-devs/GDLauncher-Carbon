import { For, createEffect, createSignal, onCleanup, onMount } from "solid-js";
import Style from "./Slider.module.scss";

interface Marks {
  [mark: number]: string;
}
export interface Props {
  min: number;
  max: number;
  value: number;
  marks: Marks;
  defaultValue: number;
  onChange?: (_val: string | undefined) => void;
}

function Slider(props: Props) {
  const [xElem, setXElem] = createSignal<number>(0);
  const [xPos, setXPos] = createSignal<number>(0);
  const [newPos, setNewPos] = createSignal<number>(0);
  const [dragging, setDragging] = createSignal(false);

  let handleRef: HTMLDivElement;
  let sliderRef: HTMLDivElement;

  createEffect(() => {
    setNewPos(xPos() - xElem());
  });

  const mousedown = (e: MouseEvent) => {
    e.preventDefault();
    setDragging(true);
    setXElem(xPos() - handleRef.offsetLeft);
  };

  const mousemove = (e: MouseEvent) => {
    setXPos(e.pageX);

    if (dragging()) {
      const offsetWidth = sliderRef.offsetWidth;

      if (newPos() < 0) {
        setNewPos(0);
      }

      if (newPos() > offsetWidth) {
        setNewPos(offsetWidth);
      }

      // if (newPos % 10 === 0) {
      //   handleRef.style.left = newPos + "px";
      // }
      handleRef.style.transform = newPos() + "px";
    }
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

  return (
    <div class="relative">
      <For each={Object.entries(props.marks)}>
        {([value]) => (
          <div
            class="w-2 h-2 bg-primary rounded-full border-4 border-solid border-primary -top-1"
            style={{
              position: "absolute",
              left: `${value}%`,
            }}
          />
        )}
      </For>
      <div
        ref={(el) => {
          handleRef = el;
        }}
        class="w-4 h-4 bg-shade-8 rounded-full border-4 border-solid border-primary -top-2 cursor-move"
        style={{
          position: "absolute",
          left: `${props.defaultValue}px`,
        }}
      />
      <div
        ref={(el) => {
          sliderRef = el;
        }}
        class={`${Style.slider} ${Style.sliderProgress} w-full h-2 bg-primary rounded-full`}
      />
    </div>
  );
}

export { Slider };
