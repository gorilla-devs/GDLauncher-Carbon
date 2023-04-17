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
  const [isMouseMoving, setIsMouseMoving] = createSignal(false);

  let horizontalSlider: HTMLDivElement;
  let scrollRightArrowContainer: HTMLSpanElement;
  let scrollLeftArrowContainer: HTMLSpanElement;
  let scrollLeftArrow: HTMLDivElement;
  let scrollRightArrow: HTMLDivElement;
  let scrollLeftArrowIcon: HTMLElement;
  let scrollRightArrowIcon: HTMLElement;

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

      if (horizontalSlider.scrollLeft > prevScrollLeft()) {
        return (horizontalSlider.scrollLeft +=
          positionDiff() > firstImageWidth / 3 ? diff : -positionDiff());
      }
      horizontalSlider.scrollLeft -=
        positionDiff() > firstImageWidth / 3 ? diff : -positionDiff();
    }
  };

  const showHideArrows = () => {
    let scrollWitdh =
      horizontalSlider.scrollWidth - horizontalSlider.clientWidth;

    if (horizontalSlider.scrollLeft === 0) {
      (scrollLeftArrowContainer as HTMLElement).classList.remove(
        "cursor-pointer"
      );
      (scrollLeftArrow as HTMLElement).classList.add("pointer-events-none");
      scrollLeftArrowIcon?.classList.add("text-darkSlate-500");
    } else {
      (scrollLeftArrowContainer as HTMLElement).classList.add("cursor-pointer");
      (scrollLeftArrow as HTMLElement).classList.remove("pointer-events-none");
      scrollLeftArrowIcon?.classList.remove("text-darkSlate-500");
    }

    if (horizontalSlider.scrollLeft === scrollWitdh) {
      (scrollRightArrowContainer as HTMLElement).classList.remove(
        "cursor-pointer"
      );
      (scrollRightArrow as HTMLElement).classList.add("pointer-events-none");
      scrollRightArrowIcon?.classList.add("text-darkSlate-500");
    } else {
      (scrollRightArrowContainer as HTMLElement).classList.add(
        "cursor-pointer"
      );
      (scrollRightArrow as HTMLElement).classList.remove("pointer-events-none");
      scrollRightArrowIcon?.classList.remove("text-darkSlate-500");
    }
  };

  const mousedown = (e: MouseEvent) => {
    setIsMouseDown(true);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    setPrevPageX(e.pageX);
    setPrevScrollLeft(horizontalSlider.scrollLeft);
  };

  const mouseup = () => {
    setIsMouseDown(false);
    setTimeout(() => {
      setIsMouseMoving(false);
    }, 100);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    horizontalSlider?.classList.remove("cursor-grabbing");
    horizontalSlider?.classList.add("scroll-smooth");
    autoSlide();
  };

  const mouseleave = () => {
    setIsMouseDown(false);
    setIsMouseMoving(false);
    horizontalSlider.classList.remove("snap-x", "snap-mandatory");
    horizontalSlider?.classList.remove("cursor-grabbing");
    horizontalSlider?.classList.add("scroll-smooth");
  };

  const mousemove = (e: MouseEvent) => {
    if (!isMouseDown()) return;
    e.preventDefault();
    setIsMouseMoving(true);
    horizontalSlider?.classList.remove("scroll-smooth");
    horizontalSlider?.classList.add("cursor-grabbing");
    setPositionDiff(e.pageX - prevPageX());
    horizontalSlider.scrollLeft = prevScrollLeft() - positionDiff();
  };

  const wheel = () => {
    horizontalSlider.classList.add("snap-x", "snap-mandatory");
    showHideArrows();
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

  onMount(() => {
    showHideArrows();
  });

  const handleScroll = (direction: string) => {
    const isLeft = direction === "left";

    const firstImage = horizontalSlider.querySelectorAll(".slide")[0];
    if (firstImage) {
      const firstImageWidth = firstImage.clientWidth + 16;

      if (isLeft) {
        horizontalSlider.scrollLeft -= firstImageWidth;
      } else {
        horizontalSlider.scrollLeft += firstImageWidth;
      }
    }
  };

  const mappedChildren = children(() => props.children);
  createEffect(() => {
    (mappedChildren() as JSX.Element[])?.forEach((item) => {
      (item as HTMLElement).classList.add("slide", "snap-start");
      if (isMouseMoving()) {
        (item as HTMLElement).classList.add("pointer-events-none");
      } else {
        (item as HTMLElement).classList.remove("pointer-events-none");
      }
    });
  });

  return (
    <div class="flex flex-col w-full">
      <div class="flex justify-between items-center h-9 w-full">
        <h3 class="uppercase">{props.title}</h3>
        <div class="h-full flex gap-4">
          <span
            ref={(el) => {
              scrollLeftArrowContainer = el;
            }}
          >
            <div
              ref={(el) => {
                scrollLeftArrow = el;
              }}
              class="h-6 w-6 bg-darkSlate-700 rounded-full flex justify-center items-center cursor-pointer"
              onClick={() => handleScroll("left")}
            >
              <i
                class="i-ri:arrow-drop-left-line text-4xl"
                ref={(el) => {
                  scrollLeftArrowIcon = el;
                }}
              />
            </div>
          </span>
          <span
            ref={(el) => {
              scrollRightArrowContainer = el;
            }}
          >
            <div
              ref={(el) => {
                scrollRightArrow = el;
              }}
              class="h-6 w-6 bg-darkSlate-700 rounded-full flex justify-center items-center cursor-pointer"
              onClick={() => handleScroll("right")}
            >
              <i
                class="i-ri:arrow-drop-right-line text-4xl"
                ref={(el) => {
                  scrollRightArrowIcon = el;
                }}
              />
            </div>
          </span>
        </div>
      </div>
      <div class="w-full flex gap-4">
        <div
          ref={(el) => {
            horizontalSlider = el;
          }}
          onScroll={() => {
            showHideArrows();
          }}
          class="scrollbar-hide w-full flex items-start gap-4 overflow-x-scroll scroll-smooth cursor-pointer"
        >
          {mappedChildren}
        </div>
      </div>
    </div>
  );
};

export { Carousel };
