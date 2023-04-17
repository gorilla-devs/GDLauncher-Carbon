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
  disableRedirect?: boolean;
  alignment?: string;
  onClick?: any;
  onSlideClick?: (_news: SlideProps) => void;
};

export interface CarouselProps {
  slides: SlideProps[];
  speed?: number;
  rtl?: boolean;
  disableAutoRotation?: boolean;
  showArrows?: boolean;
  showIndicators?: boolean;
  disableRedirect?: boolean;
  onClick?: (_news: SlideProps) => void;
}

const News = (props: CarouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(0);
  let interval: any;

  const mergedProps = mergeProps(
    { showIndicators: true, showArrows: true, rtl: true },
    props
  );
  let slidesRef: HTMLDivElement;

  const Slider = (props: SliderProps) => {
    return (
      <div ref={slidesRef} id="slider" class="flex h-80">
        <For each={props.slides}>
          {(slide) => (
            <div
              class="absolute inset-0 transition-all transform min-h-80 w-full flex justify-center items-center hidden box-border bg-no-repeat	bg-center bg-cover cursor-pointer"
              style={{
                "background-image": `url('${slide.image}')`,
              }}
              onClick={() => props.onSlideClick?.(slide)}
            >
              <div
                class="absolute bottom-0 left-0 right-0 top-0"
                style={{
                  background:
                    "linear-gradient(180deg, rgba(29, 32, 40, 0) 0%, #1D2028 100%)",
                }}
              />
              <div class="absolute bottom-10 left-5 flex flex-col select-none">
                <h2 class="mb-0">{slide.title}</h2>
                <p class="mt-2 text-darkSlate-50">{slide.description}</p>
              </div>
            </div>
          )}
        </For>
      </div>
    );
  };

  onMount(() => {
    const slides = slidesRef?.children as HTMLCollection;

    const leftSlide = () =>
      currentImageIndex() === 0
        ? slides[slides.length - 1]
        : slides[currentImageIndex() - 1];
    const middleSlide = () => slides[currentImageIndex()];
    const rightSlide = () =>
      currentImageIndex() === slides.length - 1
        ? slides[0]
        : slides[currentImageIndex() + 1];

    leftSlide().classList.add("-translate-x-full", "z-10");

    middleSlide().classList.add("translate-x-0", "z-20");

    rightSlide().classList.add("translate-x-full", "z-10");
  });

  createEffect(() => {
    interval = setInterval(() => {
      if (!props.disableAutoRotation) {
        changeSlide(mergedProps.rtl ? "right" : "left");
      }
    }, props.speed || 5000);
  });

  onCleanup(() => clearInterval(interval));

  const slideInto = (slides: HTMLCollection, position: number) => {
    const leftSlide = () =>
      position === 0 ? slides[slides.length - 1] : slides[position - 1];
    const middleSlide = () => slides[position];
    const rightSlide = () =>
      position === slides.length - 1 ? slides[0] : slides[position + 1];

    leftSlide().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-20"
    );
    leftSlide().classList.add("-translate-x-full", "z-10");
    middleSlide().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-10"
    );
    middleSlide().classList.add("translate-x-0", "z-20");
    rightSlide().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-20"
    );
    rightSlide().classList.add("translate-x-full", "z-10");
    setCurrentImageIndex(position);
  };

  const changeSlide = (direction: "right" | "left") => {
    clearInterval(interval);
    interval = setInterval(() => {
      changeSlide("right");
    }, props.speed || 5000);

    const slides = slidesRef?.children as HTMLCollection;

    const isRight = direction === "right";

    const isNotLastElement = isRight
      ? currentImageIndex() < props.slides.length - 1
      : currentImageIndex() > 0;

    if (!slides) return;
    if (isNotLastElement) {
      slideInto(
        slides,
        isRight ? currentImageIndex() + 1 : currentImageIndex() - 1
      );
    } else {
      slideInto(slides, isRight ? 0 : props.slides.length - 1);
    }
  };

  return (
    <div class="h-80 bg-darkSlate-900 rounded-lg relative overflow-hidden relative">
      <Show when={mergedProps.showArrows}>
        <div
          class="h-7 w-7 bg-darkSlate-800 rounded-full absolute left-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40 "
          onClick={() => changeSlide("left")}
        >
          <div class="i-ri:arrow-drop-left-line text-3xl text-white" />
        </div>
        <div
          class="h-7 w-7 bg-darkSlate-800 rounded-full absolute right-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40"
          onClick={() => changeSlide("right")}
        >
          <div class="i-ri:arrow-drop-right-line text-3xl text-white" />
        </div>
      </Show>
      <Show when={mergedProps.showIndicators}>
        <div class="flex justify-between items-center gap-2 z-50 absolute bottom-4 left-1/2 -translate-x-1/2">
          <For each={props.slides}>
            {(_, i) => (
              <div
                class={`w-2 h-2 bg-white rounded-full cursor-pointer ${
                  currentImageIndex() === i() ? "opacity-100" : "opacity-30"
                }`}
                onClick={() => {
                  const slides = slidesRef?.children as HTMLCollection;

                  if (slides) slideInto(slides, i());
                }}
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.slides}>
        <Slider
          currentImageIndex={currentImageIndex()}
          slides={props.slides}
          onSlideClick={(news) => props?.onClick?.(news)}
        />
      </Show>
    </div>
  );
};

export { News };
