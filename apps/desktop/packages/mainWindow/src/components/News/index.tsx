import { createEffect, createSignal, For, onCleanup } from "solid-js";
// import { styled } from "solid-styled-components";

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

interface selectNewsProps {
  slides: slideProps[];
  height?: number;
  onChange?: any;
  setCurrentImageIndex: any;
  currentImageIndex: number;
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
    console.log("Slider", props.currentImageIndex);
  });
  return (
    <div
      class="flex"
      style={{
        transform: `translate3d(
        ${-100 * (props.currentImageIndex || 0)}%,
        0,
        0
      )`,
        transition: "transform 0.3s ease-in-out",
      }}
    >
      <For each={props.slides}>
        {(slide) => (
          <div class="min-h-80 min-w-185 flex justify-center items-center bg-red-300">
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

  //   createEffect(() => {
  //     intervaL = setInterval(() => {
  //       //   const isNotLastElement = rtl
  //       //     ? currentImageIndex < slides.length - 1
  //       //     : currentImageIndex > 0;
  //       const isNotLastElement = currentImageIndex() < props.slides.length - 1;

  //       if (isNotLastElement) {
  //         setCurrentImageIndex(currentImageIndex() + 1);
  //       } else setCurrentImageIndex(0);
  //     }, 5000);
  //   });

  const changeSlide = (direction: string) => {
    const right = direction === "right";

    const isNotLastElement = right
      ? currentImageIndex() < props.slides.length - 1
      : currentImageIndex() > 0;

    console.log("TEST", direction, isNotLastElement);

    if (isNotLastElement) {
      if (right) setCurrentImageIndex(currentImageIndex() + 1);
      else setCurrentImageIndex(currentImageIndex() - 1);
    } else setCurrentImageIndex(right ? 0 : props.slides.length - 1);
  };

  onCleanup(() => clearInterval(intervaL));

  return (
    <div class="h-80 w-185 bg-green-400 rounded-lg relative overflow-hidden">
      <div
        class="h-7 w-7 bg-black-black rounded-full absolute left-0 flex justify-center items-center cursor-pointer z-10"
        onClick={() => changeSlide("left")}
      >
        <div class="i-ri:arrow-drop-left-line text-3xl" />
      </div>
      <div
        class="h-7 w-7 bg-black-black rounded-full absolute right-0 flex justify-center items-center cursor-pointer z-10"
        onClick={() => changeSlide("right")}
      >
        <div class="i-ri:arrow-drop-right-line text-3xl" />
      </div>
      <Slider currentImageIndex={currentImageIndex()} slides={props.slides} />
    </div>
  );
};

export default News;
