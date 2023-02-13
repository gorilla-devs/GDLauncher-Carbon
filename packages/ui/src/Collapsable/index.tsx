import { JSX, createSignal } from "solid-js";

type Props = {
  children: JSX.Element;
  title?: string;
};

const Collapsable = (props: Props) => {
  const [opened, setOpened] = createSignal(true);

  return (
    <div class="w-full box-border flex flex-col py-2 overflow-hidden">
      <div
        class="max-w-full h-8 flex gap-2 items-center cursor-pointer"
        onClick={() => {
          setOpened((prev) => !prev);
        }}
      >
        <div
          class="text-2xl transition ease-in-out i-ri:arrow-down-s-line text-shade-1"
          classList={{
            "-rotate-180": !opened(),
          }}
        />
        <p class="m-0 text-shade-1 flex items-center">{props.title}</p>
      </div>
      <div
        class="overflow-hidden"
        classList={{
          "h-auto": opened(),
          "h-0": !opened(),
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export { Collapsable };
