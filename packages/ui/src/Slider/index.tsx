import { For, createEffect, createSignal, onCleanup, onMount } from "solid-js";

interface Marks {
  [mark: number]: string;
}
export interface Props {
  min: number;
  max: number;
  steps: number;
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
    let elementPercentage =
      (handleRef.offsetLeft * 100) / sliderRef.offsetWidth;
    setXElem(xPos() - elementPercentage);
  };

  const mousemove = (e: MouseEvent) => {
    const offsetWidth = sliderRef.offsetWidth;
    let pageXPercentage = (e.pageX / offsetWidth) * 100;

    setXPos(pageXPercentage);

    if (dragging()) {
      if (newPos() < 0) {
        setNewPos(0);
      }

      if (newPos() > 100) {
        setNewPos(100);
      }

      if (newPos() % (props.steps || 1) === 0) {
        handleRef.style.left = newPos() + "px";
      }
      handleRef.style.left = newPos() + "%";
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
        {([value, label]) => (
          <span
            class="flex flex-col -top-1"
            style={{
              position: "absolute",
              left: `${value}%`,
            }}
          >
            <div class="w-2 h-2 bg-primary rounded-full border-4 border-solid border-primary" />
            <span class="mt-4">{label}</span>
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
          left: `${props.defaultValue}%`,
          transform: `translateX(-50%)`,
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
