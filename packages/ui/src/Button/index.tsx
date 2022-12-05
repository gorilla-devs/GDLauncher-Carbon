import { children, Show } from "solid-js";

interface Props {
  children: HTMLElement | string;
  class?: string;
  type?: string;
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  /* eslint-disable no-unused-vars */
  onClick?: (e: MouseEvent) => void;
}

function Button(props: Props) {
  const c = children(() => props.children);

  const isDisabled = () => props.disabled;
  const isOutline = () => props.type === "outline";
  const isPrimary = () => props.type === "primary" || !props.type;
  const isSecondary = () => props.type === "secondary";
  const isGlow = () => props.type === "glow";

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max py-4 px-8 rounded-full cursor-pointer uppercase font-bold flex gap-2 ${
        props.class || ""
      }`}
      classList={{
        "bg-black-black text-black-gray": isDisabled() && !isGlow(),
        "border-1 bg-black-black": isOutline(),
        "border-white hover:border-accent-hover hover:text-accent-hover":
          isOutline() && !isDisabled(),
        "border-1 hover:border-white border-black-semiblack":
          isSecondary() && !isDisabled(),
        "bg-accent-main hover:bg-accent-hover": isPrimary() && !isDisabled(),
        "border-1 border-black-semiblack":
          (isSecondary() && isDisabled()) || isOutline(),
        "text-black-semiblack": isDisabled() && isOutline(),
        "text-white": !isDisabled(),
        "flex-row-reverse": props.iconRight,
        "shadow-md shadow-accent-main bg-accent-main hover:shadow-lg hover:bg-accent-hover":
          isGlow() && !isDisabled(),
        "bg-black-gray text-black-lightGray": isGlow() && isDisabled(),
      }}
      onClick={(e) => props.onClick?.(e)}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {c()}
    </div>
  );
}

export { Button };
