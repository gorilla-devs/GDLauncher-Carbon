import { cva, type VariantProps } from "class-variance-authority";
import { createSignal, Show } from "solid-js";
import { ButtonDropdown, type ButtonDropdownProps } from "./ButtonDropdown";
import { type JSX } from "solid-js";
import Separator from "./Separator.astro";
import Apple from "../../assets/Apple";
import Windows from "../../assets/Windows";
import Linux from "../../assets/Linux";

interface Props {
  transparent?: boolean;
  children: JSX.Element | JSX.Element[] | string;
  onClick?: () => void;
  isDropdown?: boolean;
  icon?: JSX.Element | JSX.Element[] | string;
  items?: Array<{
    item: JSX.Element | JSX.Element[] | string;
    onClick: () => void;
  }>;
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
        "gap-2",
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

export interface ButtonProps
  extends JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof button> {}

const Button = (props: ButtonProps & Props) => {
  const [items, showItems] = createSignal(false);
  const intent = props.intent;
  const size = props.size;
  const className = props.class;
  return (
    <button
      onClick={() => {
        if (props.isDropdown) {
          showItems(!items());
        }
        if (props.onClick) props.onClick();
      }}
      class={button({ intent, size, className })}
    >
      {props.children}

      <Show when={props.isDropdown && items()}>
        <ButtonDropdown
          items={
            props.items as Array<{
              item: Element | string;
              onClick: () => void;
            }>
          }
        />
      </Show>
    </button>
  );
};
export default Button;
