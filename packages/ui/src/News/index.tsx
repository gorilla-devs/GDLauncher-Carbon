import {
  createEffect,
  createSignal,
  For,
  mergeProps,
  onCleanup,
  onMount,
  Show,
} from "solid-js";

interface slideProps {
  image: string;
  title: string;
  description: string;
  url?: string;
}

interface sliderProps {
  currentImageIndex: number;
  slides: slideProps[];
  disableRedirect?: boolean;
  alignment?: string;
  onClick?: any;
}

interface carouselProps {
  slides: slideProps[];
  speed?: number;
  rtl?: boolean;
  disableAutoRotation?: boolean;
  showArrows?: boolean;
  showIndicators?: boolean;
  disableRedirect?: boolean;
}

const Slider = (props: sliderProps) => {
  return (
    <div id="slider" class="flex">
      <For each={props.slides}>
        {(slide) => (
          <div
            class={`absolute inset-0 transition-all transform min-h-80 w-full flex justify-center items-center hidden box-border bg-[url("${slide.image}")]`}
          >
            <div class="absolute bottom-10 left-5 flex flex-col">
              <h2 class="mb-0">{slide.title}</h2>
              <p class="mt-2">{slide.description}</p>
            </div>
          </div>
        )}
      </For>
    </div>
  );
};

const News = (props: carouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(0);
  let interval: any;

  const mergedProps = mergeProps(
    { showIndicators: true, showArrows: true, rtl: true },
    props
  );
  let slides: HTMLCollection;

  onMount(() => {
    slides = document.getElementById("slider")?.children as HTMLCollection;

    const left = () =>
      currentImageIndex() === 0
        ? slides[slides.length - 1]
        : slides[currentImageIndex() - 1];
    const middle = () => slides[currentImageIndex()];
    const right = () =>
      currentImageIndex() === slides.length - 1
        ? slides[0]
        : slides[currentImageIndex() + 1];

    left().classList.add("-translate-x-full", "z-10");

    middle().classList.add("translate-x-0", "z-20");

    right().classList.add("translate-x-full", "z-10");
  });

  createEffect(() => {
    interval = setInterval(() => {
      changeSlide(mergedProps.rtl ? "right" : "left");
    }, props.speed || 5000);
  });

  onCleanup(() => clearInterval(interval));

  const slideInto = (slides: HTMLCollection, position: number) => {
    const left = () =>
      position === 0 ? slides[slides.length - 1] : slides[position - 1];
    const middle = () => slides[position];
    const right = () =>
      position === slides.length - 1 ? slides[0] : slides[position + 1];

    left().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-20"
    );
    left().classList.add("-translate-x-full", "z-10");
    middle().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-10"
    );
    middle().classList.add("translate-x-0", "z-20");
    right().classList.remove(
      "-translate-x-full",
      "translate-x-full",
      "translate-x-0",
      "hidden",
      "z-20"
    );
    right().classList.add("translate-x-full", "z-10");
    setCurrentImageIndex(position);
  };

  const changeSlide = (direction: "right" | "left") => {
    clearInterval(interval);
    interval = setInterval(() => {
      changeSlide("right");
    }, props.speed || 5000);

    const right = direction === "right";

    const isNotLastElement = right
      ? currentImageIndex() < props.slides.length - 1
      : currentImageIndex() > 0;

    if (!slides) return;
    if (isNotLastElement) {
      slideInto(
        slides,
        right ? currentImageIndex() + 1 : currentImageIndex() - 1
      );
    } else {
      slideInto(slides, right ? 0 : props.slides.length - 1);
    }
  };

  return (
    <div class="h-80 bg-green-400 rounded-lg relative overflow-hidden relative">
      <Show when={mergedProps.showArrows}>
        <div
          class="h-7 w-7 bg-black-black rounded-full absolute left-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40 "
          onClick={() => changeSlide("left")}
        >
          <div class="i-ri:arrow-drop-left-line text-3xl text-white" />
        </div>
        <div
          class="h-7 w-7 bg-black-black rounded-full absolute right-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40"
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
                  if (slides) slideInto(slides, i());
                }}
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.slides}>
        <Slider currentImageIndex={currentImageIndex()} slides={props.slides} />
      </Show>
    </div>
  );
};

export { News };
