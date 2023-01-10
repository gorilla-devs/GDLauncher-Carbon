import { createSignal, onMount } from "solid-js";
import "./index.css";

interface Props {
  children?: HTMLElement | Element | string | any;
  class?: string;
  title: string;
}

const Carousel = (props: Props) => {
  // const [currentSlide, setCurrentSlide] = createSignal(MockCarousel[0]);
  const [currentSlide, setCurrentSlide] = createSignal(0);
  const [isDown, setIsDown] = createSignal(false);
  const [startX, setStartX] = createSignal(null);

  onMount(() => {
    const slider = document.getElementById("horizontal-slider");
    const beginning = slider?.scrollLeft;
    setCurrentSlide(beginning || 0);
  });

  const handleScroll = (direction: string) => {
    // TODO: scroll on click
    const isLeft = direction === "left";
    const slider = document.getElementById("horizontal-slider");
    const wrapper = document.getElementById("scroll-wrapper");

    const scrollWidth = slider?.scrollWidth || 0;
    const scrollLeft = slider?.scrollLeft || 0;
    const width = wrapper?.getBoundingClientRect()?.width || 0;
    const offset = 10;
    const isEnd = scrollWidth - scrollLeft - width < offset;
    if (isLeft) {
      if (currentSlide() === 0) return;
      setCurrentSlide(currentSlide() - 168);
    } else {
      if (isEnd) return;
      setCurrentSlide(currentSlide() + 168);
    }
    console.log("AAA", currentSlide(), currentSlide() === 0);
    // slider?.classList.add("snap-none");

    console.log("isEnd", isEnd);
    if (slider) {
      console.log("END");
      slider.scrollLeft = currentSlide();
    }

    // if (slider) slider.style.transform = `translateX(${currentSlide()}px)`;
  };

  onMount(() => {
    const slider = document.querySelector("#horizontal-slider");
    const wrapper = document.getElementById("scroll-wrapper");

    let isDown = false;
    let startX: any;
    let scrollLeft: any;

    slider?.addEventListener("mousedown", (e) => {
      isDown = true;
      slider.classList.add("snap-none");

      // props.children().forEach((element: Element) => {
      //   console.log("element", element.classList.add("pointer-events-none"));
      // });
      startX = (e as any).pageX - (slider as any).offsetLeft;
      scrollLeft = slider.scrollLeft;
    });
    slider?.addEventListener("mouseleave", () => {
      isDown = false;
      slider.classList.remove("snap-none");
    });
    slider?.addEventListener("mouseup", () => {
      isDown = false;
      slider.classList.remove("snap-none");
    });
    slider?.addEventListener("mousemove", (e) => {
      if (!isDown) return;
      e.preventDefault();
      const x = (e as any).pageX - (slider as any).offsetLeft;
      const walk = (x - startX) * 2;
      // console.log(
      //   "walk",
      //   (e as any).pageX,
      //   walk,
      //   scrollLeft,
      //   scrollLeft - walk
      // );

      slider.scrollLeft = scrollLeft - walk;
    });
  });

  // onCleanup(() => {
  //   parent.removeEventListener("mousemove", mouseMove);
  //   parent.removeEventListener("mouseup", mouseUp);
  //   parent.removeEventListener("mouseleave", mouseLeave);
  //   parent.removeEventListener("touchmove", touchMove);
  //   parent.removeEventListener("touchend", touchEnd);
  //   parent.removeEventListener("touchcancel", touchCancel);
  // });

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
      <div id="scroll-wrapper" class="w-full flex gap-4">
        <div
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
