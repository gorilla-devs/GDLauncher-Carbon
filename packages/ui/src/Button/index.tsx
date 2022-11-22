import { children, Show } from "solid-js";

enum Type {
  primary = "primary",
  secondary = "secondary",
  outline = "outline",
  danger = "danger",
}
type Props = {
  children: HTMLElement | string;
  class?: string;
  type?: string;
  disabled?: boolean;
  icon?: JSX.Element | Element | any;
  iconRight?: boolean;
  onClick?: () => void;
};
function Button(props: Props) {
  const c = children(() => props.children);

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max py-4 px-8 rounded-full cursor-pointer uppercase font-bold flex gap-2 ${
        props.class || ""
      }`}
      classList={{
        "bg-black-black text-black-gray": props.disabled,
        "border-1 bg-black-black": props.type === "outline",
        "border-white hover:border-accent-hover hover:text-accent-hover":
          props.type === "outline" && !props.disabled,
        "border-1 hover:border-white border-black-semiblack":
          props.type === "secondary" && !props.disabled,
        "bg-accent-main hover:bg-accent-hover":
          props.type === "primary" && !props.disabled,
        "border-1 border-black-semiblack":
          (props.type === "secondary" && props.disabled) ||
          props.type === "outline",
        "text-black-semiblack": props.disabled && props.type === "outline",
        "text-white": !props.disabled,
        "flex-row-reverse": props.iconRight,
      }}
      onClick={props.onClick}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {c()}
    </div>
  );
}

export { Button };
