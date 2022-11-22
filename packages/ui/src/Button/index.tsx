import { children, Show } from "solid-js";

enum Type {
  primary = "primary",
  secondary = "secondary",
  outline = "outline",
  danger = "danger",
  glow = "glow",
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

  const isDisabled = () => props.disabled;
  const isOutline = () => props.type === "outline";

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max py-4 px-8 rounded-full cursor-pointer uppercase font-bold flex gap-2 ${
        props.class || ""
      }`}
      classList={{
        "bg-black-black text-black-gray": isDisabled() && props.type !== "glow",
        "border-1 bg-black-black": isOutline(),
        "border-white hover:border-accent-hover hover:text-accent-hover":
          isOutline() && !isDisabled(),
        "border-1 hover:border-white border-black-semiblack":
          props.type === "secondary" && !isDisabled(),
        "bg-accent-main hover:bg-accent-hover":
          props.type === "primary" && !isDisabled(),
        "border-1 border-black-semiblack":
          (props.type === "secondary" && isDisabled()) || isOutline(),
        "text-black-semiblack": isDisabled() && isOutline(),
        "text-white": !isDisabled(),
        "flex-row-reverse": props.iconRight,
        "shadow-md shadow-accent-main bg-accent-main hover:shadow-lg hover:bg-accent-hover":
          props.type === "glow" && !isDisabled(),
        "bg-black-gray text-black-lightGray":
          props.type === "glow" && isDisabled(),
      }}
      onClick={props.onClick}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {c()}
    </div>
  );
}

export { Button };
