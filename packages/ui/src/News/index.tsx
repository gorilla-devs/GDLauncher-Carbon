import {
  createEffect,
  createSignal,
  For,
  mergeProps,
  onCleanup,
  onMount,
  Show,
} from "solid-js";

type SlideProps = {
  image: string;
  title: string;
  description: string;
  url?: string;
};

type SliderProps = {
  currentImageIndex: number;
  slides: SlideProps[];
  alignment?: string;
  onClick?: any;
  fallBackImg: string | undefined;
  onSlideClick?: (_news: SlideProps) => void;
};

interface CarouselProps {
  slides: SlideProps[];
  speed?: number;
  rtl?: boolean;
  disableAutoRotation?: boolean;
  showArrows?: boolean;
  showIndicators?: boolean;
  fallBackImg?: string;
  onClick?: (_news: SlideProps) => void;
}

const News = (props: CarouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(1);
  const [isMoving, setIsMoving] = createSignal(false);
  let interval: ReturnType<typeof setInterval>;

  const mergedProps = mergeProps(
    { showIndicators: true, showArrows: true, rtl: true },
    props
  );

  let slidesRef: HTMLDivElement;

  const moveSlide = () => {
    slidesRef.style.transform = `translateX(-${currentImageIndex() * 100}%)`;
  };

  const firstSlide = () => props.slides[0];
  const lastSlide = () => props.slides[props.slides.length - 1];
  const copiedSlides = () => [lastSlide(), ...props.slides, firstSlide()];

  const resetInterval = () => {
    clearInterval(interval);
    interval = setInterval(() => {
      if (!props.disableAutoRotation) {
        changeSlide(mergedProps.rtl ? "right" : "left");
      }
    }, props.speed || 5000);
  };

  const handleTransitionEnd = () => {
    setIsMoving(false);
    if (currentImageIndex() === 0) {
      slidesRef.style.transition = `none`;
      setCurrentImageIndex(copiedSlides().length - 2);
      moveSlide();
    }
    if (currentImageIndex() === copiedSlides().length - 1) {
      slidesRef.style.transition = `none`;
      setCurrentImageIndex(1);
      moveSlide();
    }
  };

  onMount(() => {
    slidesRef.addEventListener("transitionend", handleTransitionEnd);
  });

  onCleanup(() => {
    slidesRef.removeEventListener("transitionend", handleTransitionEnd);
  });

  const changeSlide = (direction: "right" | "left") => {
    resetInterval();
    setIsMoving(true);
    slidesRef.style.transition = `transform 450ms ease-in-out`;
    if (currentImageIndex() > props.slides.length) return;
    if (direction === "right") {
      setCurrentImageIndex((prev) => prev + 1);
    } else {
      setCurrentImageIndex((prev) => prev - 1);
    }
    moveSlide();
  };

  createEffect(() => {
    resetInterval();
  });

  onCleanup(() => clearInterval(interval));

  const Slider = (props: SliderProps) => {
    onMount(() => {
      moveSlide();
    });

    return (
      <div ref={slidesRef} class="flex h-24">
        <For each={copiedSlides()}>
          {(slide) => (
            <div
              class="flex-grow flex-shrink-0 inset-0 transform min-h-24 w-full flex justify-center items-center box-border bg-no-repeat bg-center bg-cover"
              style={{
                "background-image": `url('${slide.image}'), url('${props.fallBackImg}')`,
              }}
            >
              <div
                class="absolute bottom-0 left-0 right-0 top-0"
                style={{
                  background: "rgba(29, 32, 40, 0.7)",
                }}
              />
              <div class="absolute bottom-4 left-0 px-5 flex flex-col select-none w-full box-border">
                <div
                  class="group flex gap-2 items-center cursor-pointer w-full"
                  onClick={() => props.onSlideClick?.(slide)}
                >
                  <h2 class="m-0 group-hover:underline text-ellipsis overflow-hidden whitespace-nowrap w-full">
                    {slide.title}
                  </h2>
                  <div class="peer i-ri:external-link-line" />
                </div>
                <h4
                  class="mt-2 text-lightSlate-400 text-ellipsis overflow-hidden whitespace-nowrap w-full
                "
                >
                  {slide.description}
                </h4>
              </div>
            </div>
          )}
        </For>
      </div>
    );
  };

  return (
    <div class="h-24 bg-darkSlate-900 rounded-lg relative overflow-hidden group">
      <Show when={mergedProps.showArrows}>
        <div
          class="h-7 w-7 opacity-0 group-hover:opacity-100 duration-200 ease-in-out bg-darkSlate-800 rounded-full absolute left-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40 "
          onClick={() => {
            if (isMoving()) return;
            changeSlide("left");
          }}
        >
          <div class="i-ri:arrow-drop-left-line text-3xl text-lightSlate-50" />
        </div>
        <div
          class="h-7 w-7 opacity-0 group-hover:opacity-100 duration-200 ease-in-out bg-darkSlate-800 rounded-full absolute right-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40"
          onClick={() => {
            if (isMoving()) return;
            changeSlide("right");
          }}
        >
          <div class="i-ri:arrow-drop-right-line text-3xl text-lightSlate-50" />
        </div>
      </Show>
      <Show when={mergedProps.showIndicators}>
        <div class="flex justify-between items-center gap-2 z-50 absolute bottom-4 left-1/2 -translate-x-1/2">
          <For each={copiedSlides().slice(1, -1)}>
            {(_, i) => (
              <div
                class={`w-2 h-2 bg-white rounded-full cursor-pointer ${
                  currentImageIndex() === i() + 1 ? "opacity-100" : "opacity-30"
                }`}
                onClick={() => {
                  resetInterval();
                  slidesRef.style.transition = `transform 450ms ease-in-out`;
                  setCurrentImageIndex(i() + 1);
                  moveSlide();
                }}
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.slides}>
        <Slider
          fallBackImg={props.fallBackImg}
          currentImageIndex={currentImageIndex()}
          slides={props.slides}
          onSlideClick={(news) => props?.onClick?.(news)}
        />
      </Show>
    </div>
  );
};

export { News };
