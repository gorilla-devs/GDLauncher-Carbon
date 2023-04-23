import { JSX, createSignal } from "solid-js";

type Props = {
  children: JSX.Element;
  title?: string;
  size?: "standard" | "small";
};

const Collapsable = (props: Props) => {
  const [opened, setOpened] = createSignal(true);

  return (
    <div class="w-full box-border flex flex-col py-2 overflow-hidden">
      <div
        class="max-w-full h-8 flex gap-2 items-center cursor-pointer"
        classList={{
          "px-3": props.size !== "small",
          "px-2": props.size === "small",
        }}
        onClick={() => {
          setOpened((prev) => !prev);
        }}
      >
        <div
          class="transition ease-in-out i-ri:arrow-down-s-line text-darkSlate-100"
          classList={{
            "-rotate-180": !opened(),
          }}
        />
        <p
          class="m-0 text-darkSlate-100 flex items-center uppercase"
          classList={{
            "text-md": props.size !== "small",
            "text-xs": props.size === "small",
          }}
        >
          {props.title}
        </p>
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
