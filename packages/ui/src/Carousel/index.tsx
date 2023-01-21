import {
  children,
  createEffect,
  createSignal,
  onMount,
  JSX,
  onCleanup,
} from "solid-js";
import "./index.css";

export interface Props {
  children?: JSX.Element;
  class?: string;
  title: string;
}

const Carousel = (props: Props) => {
  const [startX, setStartX] = createSignal(0);
  const [scrollLeft, setScrollLeft] = createSignal(0);
  const [isDown, setIsDown] = createSignal(false);
  let horizontalSlider: HTMLDivElement | undefined;
  let scrollWrapper: HTMLDivElement | undefined;

  onMount(() => {
    const beginning = horizontalSlider?.scrollLeft;
    if (beginning) setScrollLeft(beginning);
  });

  const mousedown = (e: MouseEvent) => {
    if (horizontalSlider) {
      setIsDown(true);
      horizontalSlider?.classList.add("snap-none");
      horizontalSlider?.classList.remove(
        "snap-x",
        "snap-mandatory",
        "scroll-smooth"
      );
      setStartX(e.pageX - horizontalSlider?.offsetLeft);
      setScrollLeft(horizontalSlider.scrollLeft);
    }
  };

  const mouseleave = () => {
    if (horizontalSlider) {
      setIsDown(false);
      horizontalSlider?.classList.remove("snap-none");
      horizontalSlider?.classList.add(
        "snap-x",
        "snap-mandatory",
        "scroll-smooth"
      );
    }
  };
  const mousemove = (e: MouseEvent) => {
    if (horizontalSlider) {
      if (!isDown()) return;
      e.preventDefault();

      const x = e.pageX - horizontalSlider.offsetLeft;
      const walk = (x - startX()) * 3;
      horizontalSlider.scrollLeft = scrollLeft() - walk;
    }
  };

  createEffect(() => {
    if (horizontalSlider) {
      horizontalSlider.addEventListener("mousedown", mousedown);
      horizontalSlider.addEventListener("mouseleave", mouseleave);
      horizontalSlider.addEventListener("mouseup", mouseleave);
      horizontalSlider.addEventListener("mousemove", mousemove);
    }
  });

  onCleanup(() => {
    if (horizontalSlider) {
      horizontalSlider.removeEventListener("mousedown", mousedown);
      horizontalSlider.removeEventListener("mouseleave", mouseleave);
      horizontalSlider.removeEventListener("mouseup", mouseleave);
      horizontalSlider.removeEventListener("mousemove", mousemove);
    }
  });

  const handleScroll = (direction: string) => {
    const isLeft = direction === "left";

    const scrollWidth = horizontalSlider?.scrollWidth || 0;
    const scrollLeftt = horizontalSlider?.scrollLeft || 0;
    const width = scrollWrapper?.getBoundingClientRect()?.width || 0;
    const offset = 10;
    const isEnd = scrollWidth - scrollLeftt - width < offset;
    const isStart = scrollLeft() === 0;

    if (isLeft) {
      if (isStart) return;
      setScrollLeft((prev) => prev - 168);
    } else {
      if (isEnd) return;
      setScrollLeft((prev) => prev + 168);
    }

    if (horizontalSlider) {
      horizontalSlider.scrollLeft = scrollLeft();
    }
  };

  return (
    <div class="flex flex-col w-full">
      <div class="flex justify-between items-center h-9 w-full">
        <h3 class="uppercase">{props.title}</h3>
        <div class="h-full flex gap-4">
          <div
            class="h-6 w-6 bg-black-semiblack rounded-full flex justify-center items-center"
            onClick={() => handleScroll("left")}
          >
            <div class="i-ri:arrow-drop-left-line text-4xl" />
          </div>
          <div
            class="h-6 w-6 bg-black-semiblack rounded-full flex justify-center items-center"
            onClick={() => handleScroll("rigth")}
          >
            <div class="i-ri:arrow-drop-right-line text-4xl" />
          </div>
        </div>
      </div>
      <div ref={scrollWrapper} id="scroll-wrapper" class="w-full flex gap-4">
        <div
          ref={horizontalSlider}
          id="horizontal-slider"
          class="w-full flex gap-4 snap-x snap-mandatory overflow-x-scroll scroll-smooth"
          // onMouseDown={(e) => {
          //   console.log("MOUSEDOWN");

          //   setIsDown(true);
          //   if (horizontalSlider) {
          //     horizontalSlider?.classList.add("snap-none");
          //     horizontalSlider?.classList.remove(
          //       "snap-x",
          //       "snap-mandatory",
          //       "scroll-smooth"
          //     );

          //     setStartX(e.pageX - horizontalSlider?.offsetLeft);
          //     setScrollLeft(horizontalSlider?.offsetLeft);
          //   }
          // }}
          // onMouseMove={(e) => {
          //   console.log("MOVE");
          //   if (horizontalSlider) {
          //     if (!isDown()) return;
          //     e.preventDefault();
          //     const x = e.pageX - horizontalSlider?.offsetLeft;
          //     const walk = (x - startX()) * 2;
          //     // setScrollLeft(horizontalSlider.scrollLeft);
          //     console.log("TEST", scrollLeft(), horizontalSlider.scrollLeft);
          //     // horizontalSlider.scrollLeft = horizontalSlider.scrollLeft - walk;
          //     horizontalSlider.scrollLeft = scrollLeft() - walk;
          //   }
          // }}
          // onMouseLeave={() => {
          //   setIsDown(false);
          //   horizontalSlider?.classList.remove("snap-none");
          //   horizontalSlider?.classList.add(
          //     "snap-x",
          //     "snap-mandatory",
          //     "scroll-smooth"
          //   );
          // }}
          // onMouseUp={() => {
          //   setIsDown(false);
          //   toggleClickEventTile(true);
          //   horizontalSlider?.classList.remove("snap-none");
          //   horizontalSlider?.classList.add(
          //     "snap-x",
          //     "snap-mandatory",
          //     "scroll-smooth"
          //   );
          // }}
        >
          {props.children}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
