import { JSX, createSignal } from "solid-js";

type Props = {
  children: JSX.Element;
  title?: string;
};

const Collapsable = (props: Props) => {
  const [opened, setOpened] = createSignal(true);

  return (
    <div
      class="w-full py-2 box-border overflow-hidden flex flex-col"
      classList={{
        "h-auto": opened(),
        "h-8": !opened(),
      }}
    >
      <div
        class="max-w-full h-8 px-3 flex gap-2 items-center cursor-pointer"
        onClick={() => {
          setOpened((prev) => !prev);
        }}
      >
        <div
          class="i-ri:arrow-down-s-line text-shade-1 text-2xl transition ease-in-out"
          classList={{
            "-rotate-180": !opened(),
          }}
        />
        <p class="m-0 text-shade-1 flex items-center">{props.title}</p>
      </div>
      {props.children}
    </div>
  );
};

export default Collapsable;
