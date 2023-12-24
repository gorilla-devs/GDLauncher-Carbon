import { cva, type VariantProps } from "class-variance-authority";
import { Show } from "solid-js";
import { ButtonDropdown } from "./ButtonDropdown";
import Separator from "./Separator.astro";
import Apple from "../../assets/Apple";
import Windows from "../../assets/Windows";
import Linux from "../../assets/Linux";

interface Props {
  transparent?: boolean;
  children: Element | string;
  onClick?: () => void;
  isDropdown?: boolean;
  icon?: Element;
}
const button = cva("button", {
  variants: {
    intent: {
      primary: [
        "bg-bluegd-500",
        "shadow-mdgd",
        "text-white",
        "hover:bg-bluegd-400",
        "rounded-smgd",
        "flex",
        "items-center",
        "justify-center",
        "relative",
      ],
      secondary: ["bg-bluegd-600", "text-white", "rounded-xsgd"],
      transparent: [
        "bg-transparent",
        "text-bluegd-500",
        "border-solid",
        "border-[1px]",
        "border-bluegd-500",
        "rounded-smgd",
      ],
    },
    size: {
      small: ["text-sm", "py-2", "px-3"],
      medium: ["text-base", "py-4", "px-10"],
    },
  },
  compoundVariants: [{ intent: "primary", size: "medium" }],
  defaultVariants: {
    intent: "primary",
    size: "medium",
  },
});

const items: Array<{ item: Element | string; onClick: () => void }> = [
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Apple /> MacOS
      </div>
    ) as Element,
    onClick: () => {},
  },
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Windows /> Windows
      </div>
    ) as Element,
    onClick: () => {},
  },
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Linux /> Linux
      </div>
    ) as Element,
    onClick: () => {},
  },
];

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof button> {}

const Button = (props: ButtonProps & Props) => {
  const intent = props.intent;
  const size = props.size;
  const className = props.className;
  return (
    <button onClick={props.onClick} class={button({ intent, size, className })}>
      {props.children}

      <Show when={props.isDropdown}>
        <ButtonDropdown items={items} />
      </Show>
    </button>
  );
};
export default Button;
