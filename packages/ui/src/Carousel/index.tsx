import {
  createEffect,
  createSignal,
  onMount,
  JSX,
  onCleanup,
  children,
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
  const [isDragging, setIsDragging] = createSignal(false);
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
      setIsDragging(false);

      const slide = document.querySelector(".slide.non-clickable");

      slide?.classList.remove("non-clickable");

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
      setIsDragging(true);

      const slide = document.querySelector(".slide");

      slide?.classList.add("non-clickable");

      const x = e.pageX - horizontalSlider.offsetLeft;
      const walk = (x - startX()) * 3;
      horizontalSlider.scrollLeft = scrollLeft() - walk;
    }
  };

  const preventClick = (e: Event) => {
    e.preventDefault();
  };

  createEffect(() => {
    if (horizontalSlider) {
      horizontalSlider.addEventListener("mousedown", mousedown);
      horizontalSlider.addEventListener("mouseleave", mouseleave);
      horizontalSlider.addEventListener("mouseup", mouseleave);
      horizontalSlider.addEventListener("mousemove", mousemove);
    }
  });

  createEffect(() => {
    const nonClickableElement = document.querySelector(".slide.non-clickable");
    if (nonClickableElement) {
      nonClickableElement.addEventListener("click", preventClick);
    }
  });

  onCleanup(() => {
    if (horizontalSlider) {
      horizontalSlider.removeEventListener("mousedown", mousedown);
      horizontalSlider.removeEventListener("mouseleave", mouseleave);
      horizontalSlider.removeEventListener("mouseup", mouseleave);
      horizontalSlider.removeEventListener("mousemove", mousemove);
    }

    const nonClickableElement = document.querySelector(".slide.non-clickable");
    if (nonClickableElement) {
      nonClickableElement.addEventListener("click", preventClick);
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

  const mappedChildren = children(() => props.children);
  createEffect(() => {
    (mappedChildren() as JSX.Element[])?.forEach((item) =>
      (item as HTMLElement).classList.add("slide")
    );
  });

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
        >
          {mappedChildren}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
