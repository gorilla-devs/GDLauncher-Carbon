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
  const [positionDiff, setPositionDiff] = createSignal(0);
  const [prevPageX, setPrevPageX] = createSignal(0);
  const [prevScrollLeft, setPrevScrollLeft] = createSignal(0);
  const [scrollLeft, setScrollLeft] = createSignal(0);
  const [isMouseDown, setIsMouseDown] = createSignal(false);

  let horizontalSlider: HTMLDivElement | undefined;
  let scrollWrapper: HTMLDivElement | undefined;

  onMount(() => {
    // let scrollWitdh =
    // horizontalSlider?.scrollWidth - horizontalSlider?.clientWidth;
    const beginning = horizontalSlider?.scrollLeft;
    if (beginning) setScrollLeft(beginning);
  });

  const autoSlide = () => {
    setPositionDiff((prev) => Math.abs(prev));
    const firstImage = horizontalSlider?.querySelectorAll(".slide")[0];
    if (firstImage && horizontalSlider) {
      const firstImageWidth = firstImage.clientWidth + 16;
      const diff = firstImageWidth - positionDiff();

      if (horizontalSlider.scrollLeft > prevScrollLeft()) {
        return (horizontalSlider.scrollLeft +=
          positionDiff() > firstImageWidth / 3 ? diff : -positionDiff());
      } else
        return (horizontalSlider.scrollLeft -=
          positionDiff() > firstImageWidth / 3 ? diff : -positionDiff());
    }
  };

  const mousedown = (e: MouseEvent) => {
    if (horizontalSlider) {
      setIsMouseDown(true);

      setPrevPageX(e.pageX);
      setPrevScrollLeft(horizontalSlider.scrollLeft);
    }
  };

  const mouseleave = () => {
    if (horizontalSlider) {
      setIsMouseDown(false);
      const nonClickableElement = document.querySelector(".non-clickable");

      if (nonClickableElement) {
        nonClickableElement.classList.remove("non-clickable");
      }

      horizontalSlider?.classList.add("scroll-smooth");
      autoSlide();
    }
  };

  const mousemove = (e: MouseEvent) => {
    if (horizontalSlider) {
      if (!isMouseDown()) return;
      e.preventDefault();
      horizontalSlider?.classList.remove("scroll-smooth");
      setPositionDiff(e.pageX - prevPageX());
      horizontalSlider.scrollLeft = prevScrollLeft() - positionDiff();
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

  const mappedChildren = children(() => props.children);
  createEffect(() => {
    (mappedChildren() as JSX.Element[])?.forEach((item) => {
      (item as HTMLElement).classList.add("slide");
      (item as HTMLElement).onmouseover = () => {
        (item as HTMLElement).classList.add("pointer-events-none");
      };
    });
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
          class="w-full flex gap-4 overflow-x-scroll scroll-smooth"
        >
          {mappedChildren}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
