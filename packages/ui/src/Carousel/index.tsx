import "./index.css";

export interface Props {
  children?: HTMLElement | Element | string | any;
  class?: string;
  title: string;
}

const Carousel = (props: Props) => {
  // const [currentSlide, setCurrentSlide] = createSignal(MockCarousel[0]);

  // const handleScroll = () => {
  // TODO: scroll on click
  // const isLeft = direction === "left";
  // const element = document.getElementById("content");
  // };

  return (
    <div class="flex flex-col w-full">
      <div class="flex justify-between items-center h-9 w-full">
        <h3 class="uppercase">{props.title}</h3>
        <div class="h-full flex gap-4">
          <div
            class="h-6 w-6 bg-black-semiblack rounded-full flex justify-center items-center"
            // onClick={() => handleScroll("left")}
          >
            <div class="i-ri:arrow-drop-left-line text-4xl" />
          </div>
          <div
            class="h-6 w-6 bg-black-semiblack rounded-full flex justify-center items-center"
            // onClick={() => handleScroll("rigth")}
          >
            <div class="i-ri:arrow-drop-right-line text-4xl" />
          </div>
        </div>
      </div>
      <div class="w-full flex gap-4 overflow-x-scroll instancesScroll">
        {props.children}
      </div>
    </div>
  );
};

export { Carousel };
