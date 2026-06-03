import { cva, type VariantProps } from "class-variance-authority";
import { splitProps, type JSX } from "solid-js";

const button = cva("button", {
  variants: {
    intent: {
      primary: [
        "bg-bluegd-500",
        "shadow-mdgd",
        "text-white",
        "hover:bg-bluegd-400 transition-all duration-300 ease-in-out",
        "rounded-smgd",
        "flex",
        "gap-2",
        "justify-center",
        "relative",
        "active:scale-95",
        "ease-spring",
      ],
      secondary: ["bg-bluegd-600", "text-white", "rounded-xsgd", "active:scale-95", "ease-spring", "transition-transform", "duration-100"],
      transparent: [
        "bg-transparent",
        "text-bluegd-500",
        "border-solid",
        "border-[1px]",
        "border-bluegd-500",
        "rounded-smgd",
        "active:scale-95",
        "ease-spring",
        "transition-transform",
        "duration-100",
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

export type ButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof button> & {
    /** Optional icon slot (rendered before children). */
    icon?: JSX.Element | JSX.Element[] | string;
  };

const Button = (props: ButtonProps) => {
  // splitProps so the variant inputs stay reactive (reading from `props` keeps
  // the cva call live) and every other HTML attribute (type, disabled,
  // aria-*, id, name, autofocus, onClick, etc.) flows through to the
  // underlying <button>. Default type="button" prevents accidental form
  // submission when dropped inside a <form>.
  const [local, others] = splitProps(props, [
    "intent",
    "size",
    "class",
    "icon",
    "children",
    "type",
  ]);

  return (
    <button
      type={local.type ?? "button"}
      class={button({
        intent: local.intent,
        size: local.size,
        className: local.class,
      })}
      {...others}
    >
      {local.icon}
      {local.children}
    </button>
  );
};
export default Button;
