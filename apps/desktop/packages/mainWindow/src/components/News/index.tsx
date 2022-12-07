import { createEffect, createSignal, For, onCleanup } from "solid-js";

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
  style?: any;
  slides: slideProps[];
  speed?: number;
  rtl?: boolean;
  disableAutoRotation?: boolean;
  showArrows?: boolean;
  showSelectMenu?: boolean;
  disableRedirect?: boolean;
  borderRadius?: number;
  height?: number;
  // width,
  alignment?: string;
  onChange?: any;
  onClick?: any;
}

const Slider = (props: sliderProps) => {
  createEffect(() => {
    console.log("Slider");
  });
  return (
    <div id="slider" class="flex">
      <For each={props.slides}>
        {(slide) => (
          <div class="absolute inset-0 transition-all transform min-h-80 min-w-185 flex justify-center items-center bg-red-300 hidden">
            <h2>{slide.title}</h2>
          </div>
        )}
      </For>
    </div>
  );
};

const News = (props: carouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(0);
  let intervaL: any;

  createEffect(() => {
    intervaL = setInterval(() => {
      changeSlide("right");
    }, 5000);
  });

  const slideInto = (slides: HTMLCollection, position: number) => {
    const left = () =>
      position === 0 ? slides[slides.length - 1] : slides[position - 1];
    const middle = () => slides[position];
    const right = () =>
      position === slides.length - 1 ? slides[0] : slides[position + 1];

    console.log("slideInto", position, left(), middle(), right());
    for (let slide of slides) {
      slide.classList.add("hidden");
    }
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

  const changeSlide = (direction: string) => {
    const right = direction === "right";

    const slides = document.getElementById("slider")?.children;

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

  onCleanup(() => clearInterval(intervaL));

  return (
    <div class="h-80 w-185 bg-green-400 rounded-lg relative overflow-hidden relative">
      <div
        class="h-7 w-7 bg-black-black rounded-full absolute left-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40"
        onClick={() => changeSlide("left")}
      >
        <div class="i-ri:arrow-drop-left-line text-3xl" />
      </div>
      <div
        class="h-7 w-7 bg-black-black rounded-full absolute right-5 top-1/2 -translate-y-1/2 flex justify-center items-center cursor-pointer z-40"
        onClick={() => changeSlide("right")}
      >
        <div class="i-ri:arrow-drop-right-line text-3xl" />
      </div>
      <Slider currentImageIndex={currentImageIndex()} slides={props.slides} />
    </div>
  );
};

export default News;
