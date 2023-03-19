import { createEffect, createSignal, onMount, JSX, onCleanup } from "solid-js";
import "./index.css";

export interface Props {
  children: JSX.Element;
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
      // Get the current scroll position and calculate the target position
      const currentScrollLeft = horizontalSlider.scrollLeft;
      const snapDistance = 168;
      const targetScrollLeft =
        Math.round(currentScrollLeft / snapDistance) * snapDistance;
      // Animate the scroll to the target position using requestAnimationFrame
      const animateScroll = () => {
        if (horizontalSlider) {
          const distance = targetScrollLeft - horizontalSlider.scrollLeft;
          const speed = distance / 8;
          if (Math.abs(distance) < 1) {
            horizontalSlider.scrollLeft = targetScrollLeft;
            horizontalSlider.classList.add("snap-x");
            horizontalSlider.classList.remove("snap-mandatory");
          } else {
            horizontalSlider.scrollLeft += speed;
            requestAnimationFrame(animateScroll);
          }
        }
      };
      requestAnimationFrame(animateScroll);
    }
  };

  const mousemove = (e: MouseEvent) => {
    if (horizontalSlider && isDown()) {
      e.preventDefault();

      const x = e.pageX - horizontalSlider.offsetLeft;
      const walk = (x - startX()) * 3;
      const newScrollLeft = scrollLeft() - walk;

      // Calculate the target scroll position based on the snap distance
      const snapDistance = 168;
      const targetScrollLeft =
        Math.round(newScrollLeft / snapDistance) * snapDistance;

      // Update the scroll position using requestAnimationFrame for smoother scrolling
      const updateScroll = () => {
        if (!horizontalSlider) return;
        const currentScrollLeft = horizontalSlider.scrollLeft;
        const distance = targetScrollLeft - currentScrollLeft;
        const speed = distance / 8;
        if (Math.abs(distance) < 1) {
          horizontalSlider.scrollLeft = targetScrollLeft;
        } else {
          horizontalSlider.scrollLeft = currentScrollLeft + speed;
          requestAnimationFrame(updateScroll);
        }
      };
      requestAnimationFrame(updateScroll);
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
        >
          {props.children}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
