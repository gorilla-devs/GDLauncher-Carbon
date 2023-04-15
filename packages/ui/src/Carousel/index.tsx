import {
  createEffect,
  createSignal,
  JSX,
  onCleanup,
  children,
  onMount,
} from "solid-js";

export interface Props {
  children?: JSX.Element;
  class?: string;
  title: string;
}

const Carousel = (props: Props) => {
  const [positionDiff, setPositionDiff] = createSignal(0);
  const [prevPageX, setPrevPageX] = createSignal(0);
  const [prevScrollLeft, setPrevScrollLeft] = createSignal(0);
  const [isMouseDown, setIsMouseDown] = createSignal(false);

  let horizontalSlider: HTMLDivElement;

  const autoSlide = () => {
    if (
      horizontalSlider.scrollLeft ==
      horizontalSlider.scrollWidth - horizontalSlider.clientWidth
    )
      return;

    setPositionDiff((prev) => Math.abs(prev));
    const firstImage = horizontalSlider?.querySelectorAll(".slide")[0];
    if (firstImage) {
      const firstImageWidth = firstImage.clientWidth + 16;
      const diff = firstImageWidth - positionDiff();

      console.log("AAA", horizontalSlider.scrollLeft > prevScrollLeft());
      if (horizontalSlider.scrollLeft > prevScrollLeft()) {
        return (horizontalSlider.scrollLeft +=
          positionDiff() > firstImageWidth / 3 ? diff : -positionDiff());
      }
      horizontalSlider.scrollLeft -=
        positionDiff() > firstImageWidth / 3 ? diff : -positionDiff();
    }
  };

  const mousedown = (e: MouseEvent) => {
    setIsMouseDown(true);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    horizontalSlider?.classList.add("cursor-grab");
    setPrevPageX(e.pageX);
    setPrevScrollLeft(horizontalSlider.scrollLeft);
  };

  const mouseup = () => {
    setIsMouseDown(false);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    horizontalSlider?.classList.remove("cursor-grabbing", "cursor-grab");
    horizontalSlider?.classList.add("scroll-smooth");
    autoSlide();
  };

  const mouseleave = () => {
    setIsMouseDown(false);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    horizontalSlider?.classList.remove("cursor-grabbing", "cursor-grab");
    horizontalSlider?.classList.add("scroll-smooth");
  };

  const mousemove = (e: MouseEvent) => {
    if (!isMouseDown()) return;
    e.preventDefault();
    horizontalSlider?.classList.remove("scroll-smooth", "cursor-grab");
    horizontalSlider?.classList.add("cursor-grabbing");
    setPositionDiff(e.pageX - prevPageX());
    horizontalSlider.scrollLeft = prevScrollLeft() - positionDiff();
  };

  const wheel = () => {
    horizontalSlider.classList.add("snap-x", "snap-mandatory");
  };

  createEffect(() => {
    horizontalSlider.addEventListener("mousedown", mousedown);
    horizontalSlider.addEventListener("mouseleave", mouseleave);
    horizontalSlider.addEventListener("mouseup", mouseup);
    horizontalSlider.addEventListener("mousemove", mousemove);
    horizontalSlider.addEventListener("wheel", wheel);
  });

  onCleanup(() => {
    horizontalSlider.removeEventListener("mousedown", mousedown);
    horizontalSlider.removeEventListener("mouseleave", mouseleave);
    horizontalSlider.removeEventListener("mouseup", mouseup);
    horizontalSlider.removeEventListener("mousemove", mousemove);
    horizontalSlider.removeEventListener("wheel", wheel);
  });

  const showHideArrows = () => {
    let scrollWitdh =
      horizontalSlider.scrollWidth - horizontalSlider.clientWidth;

    const leftArrowContainer = document.getElementById(
      "scroll-left-arrow-container"
    );
    const rightArrowContainer = document.getElementById(
      "scroll-right-arrow-container"
    );

    const leftArrow = document.getElementById("scroll-left-arrow");
    const rightArrow = document.getElementById("scroll-right-arrow");

    if (horizontalSlider.scrollLeft === 0) {
      (leftArrowContainer as HTMLElement).classList.add("cursor-not-allowed");
      (leftArrow as HTMLElement).classList.add("pointer-events-none");
    } else {
      (leftArrowContainer as HTMLElement).classList.remove(
        "cursor-not-allowed"
      );
      (leftArrow as HTMLElement).classList.remove("pointer-events-none");
    }

    if (horizontalSlider.scrollLeft === scrollWitdh) {
      (rightArrowContainer as HTMLElement).classList.add("cursor-not-allowed");
      (rightArrow as HTMLElement).classList.add("pointer-events-none");
    } else {
      (rightArrowContainer as HTMLElement).classList.remove(
        "cursor-not-allowed"
      );
      (rightArrow as HTMLElement).classList.remove("pointer-events-none");
    }
  };

  onMount(() => {
    showHideArrows();
  });

  const handleScroll = (direction: string) => {
    const isLeft = direction === "left";

    const firstImage = horizontalSlider.querySelectorAll(".slide")[0];
    if (firstImage) {
      const firstImageWidth = firstImage.clientWidth + 16;

      if (isLeft) {
        horizontalSlider.scrollLeft += -firstImageWidth;
      } else {
        horizontalSlider.scrollLeft += firstImageWidth;
      }

      setTimeout(() => {
        showHideArrows();
      }, 60);
    }
  };

  const mappedChildren = children(() => props.children);
  createEffect(() => {
    (mappedChildren() as JSX.Element[])?.forEach((item) => {
      (item as HTMLElement).classList.add("slide", "snap-start");
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
          <span id="scroll-left-arrow-container">
            <div
              id="scroll-left-arrow"
              class="h-6 w-6 bg-shade-9 rounded-full flex justify-center items-center"
              onClick={() => handleScroll("left")}
            >
              <i class="i-ri:arrow-drop-left-line text-4xl" />
            </div>
          </span>
          <span id="scroll-right-arrow-container">
            <div
              id="scroll-right-arrow"
              class="h-6 w-6 bg-shade-9 rounded-full flex justify-center items-center"
              onClick={() => handleScroll("right")}
            >
              <i class="i-ri:arrow-drop-right-line text-4xl" />
            </div>
          </span>
        </div>
      </div>
      <div id="scroll-wrapper" class="w-full flex gap-4">
        <div
          ref={(el) => {
            horizontalSlider = el;
          }}
          id="horizontal-slider"
          // class="scrollbar-hide w-full flex gap-4 overflow-x-scroll scroll-smooth"
          class="scrollbar-hide w-full flex gap-4 overflow-x-scroll scroll-smooth"
        >
          {mappedChildren}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
