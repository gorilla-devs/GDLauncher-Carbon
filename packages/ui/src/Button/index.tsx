import { children, mergeProps, Show } from "solid-js";

export interface Props {
  children: HTMLElement | string;
  class?: string;
  type?: "primary" | "secondary" | "glow" | "outline" | "transparent";
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  uppercase?: boolean;
  size?: "small" | "medium" | "large";
  /* eslint-disable no-unused-vars */
  onClick?: (e: MouseEvent) => void;
}

function Button(props: Props) {
  const c = children(() => props.children);

  const mergedProps = mergeProps(
    { type: "primary", size: "large", uppercase: false, iconRight: false },
    props
  );

  const isDisabled = () => props.disabled;
  const isOutline = () => props.type === "outline";
  const isPrimary = () => mergedProps.type === "primary";
  const isSecondary = () => props.type === "secondary";
  const isGlow = () => props.type === "glow";

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max rounded-full cursor-pointer font-bold flex items-center gap-2 ${
        props.class || ""
      }`}
      classList={{
        "backdrop-blur-md bg-black-black": mergedProps.type === "transparent",
        "py-4 px-8": mergedProps.size === "large",
        "py-3 px-5": mergedProps.size === "medium",
        "py-2 px-4": mergedProps.size === "small",
        uppercase: mergedProps.uppercase,
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
        "bg-black-gray text-shade-0": isGlow() && isDisabled(),
      }}
      style={{
        ...(mergedProps.type === "transparent" && {
          background: "rgba(0, 0, 0, 0.4)",
        }),
      }}
      onClick={(e) => props.onClick?.(e)}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {c()}
    </div>
  );
}

export { Button };
