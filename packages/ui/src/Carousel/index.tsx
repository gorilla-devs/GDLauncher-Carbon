import { createSignal, onMount } from "solid-js";
import "./index.css";

export interface Props {
  children?: HTMLElement | Element | string | any;
  class?: string;
  title: string;
}

const Carousel = (props: Props) => {
  const [currentSlide, setCurrentSlide] = createSignal(0);
  const [startX, setStartX] = createSignal(0);
  const [scrollLeft, setScrollLeft] = createSignal(0);
  const [isDown, setIsDown] = createSignal(false);
  let horizontalSlider: HTMLDivElement | undefined;
  let scrollWrapper: HTMLDivElement | undefined;

  onMount(() => {
    const beginning = horizontalSlider?.scrollLeft;
    setCurrentSlide(beginning || 0);
  });

  const handleScroll = (direction: string) => {
    const isLeft = direction === "left";

    const scrollWidth = horizontalSlider?.scrollWidth || 0;
    const scrollLeft = horizontalSlider?.scrollLeft || 0;
    const width = scrollWrapper?.getBoundingClientRect()?.width || 0;
    const offset = 10;
    const isEnd = scrollWidth - scrollLeft - width < offset;
    const isStart = currentSlide() === 0;

    if (isLeft) {
      if (isStart) return;
      setCurrentSlide(currentSlide() - 168);
    } else {
      if (isEnd) return;
      setCurrentSlide(currentSlide() + 168);
    }

    if (horizontalSlider) {
      horizontalSlider.scrollLeft = currentSlide();
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
          onMouseDown={(e) => {
            setIsDown(true);
            horizontalSlider?.classList.add("snap-none");
            horizontalSlider?.classList.remove("snap-x", "snap-mandatory");
            const offsetLeft = horizontalSlider?.offsetLeft || 0;
            setStartX(e.pageX - offsetLeft);
            setScrollLeft(offsetLeft);
          }}
          onMouseMove={(e) => {
            if (!isDown()) return;
            e.preventDefault();
            const x = e.pageX - (horizontalSlider?.offsetLeft || 0);
            const walk = (x - startX()) * 2;

            setCurrentSlide(scrollLeft() - walk);
            if (horizontalSlider) {
              horizontalSlider.scrollLeft = currentSlide();
            }
          }}
          onMouseLeave={() => {
            setIsDown(false);
            horizontalSlider?.classList.remove("snap-none");
            horizontalSlider?.classList.add("snap-x", "snap-mandatory");
          }}
          onMouseUp={() => {
            setIsDown(false);
            horizontalSlider?.classList.remove("snap-none");
            horizontalSlider?.classList.add("snap-x", "snap-mandatory");
          }}
        >
          {props.children}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
