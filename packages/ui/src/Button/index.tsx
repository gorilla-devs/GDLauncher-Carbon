import { children, mergeProps, Show, JSX } from "solid-js";
import { Spinner } from "../Spinner";
// import "./Button.css";

export interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  children: HTMLElement | string;
  class?: string;
  variant?: "primary" | "secondary" | "glow" | "outline" | "transparent";
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  uppercase?: boolean;
  loading?: boolean;
  size?: "small" | "medium" | "large";
  /* eslint-disable no-unused-vars */
  onClick?: (e: MouseEvent) => void;
}

function Button(props: Props) {
  const c = children(() => props.children);

  const mergedProps = mergeProps(
    { variant: "primary", size: "large", uppercase: false, iconRight: false },
    props
  );

  const isDisabled = () => props.disabled;
  const isOutline = () => props.variant === "outline";
  const isPrimary = () => mergedProps.variant === "primary";
  const isSecondary = () => props.variant === "secondary";
  const isGlow = () => props.variant === "glow";

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max  font-bold flex items-center gap-2 relative ${
        props.class || ""
      }`}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {/* use windicss to clear all default button styles */}
      <button
        onClick={(e) => props.onClick?.(e)}
        class="border-0 m-0 p-0 overflow-visible cursor-pointer transition-transform transform-gpu origin-center duration-300 rounded-full"
        style={{
          "font-family": "inherit",
          "font-size": "inherit",
          "font-style": "inherit",
          "font-weight": "inherit",
          "line-height": "inherit",
          ...(mergedProps.variant === "transparent" && {
            background: "rgba(0, 0, 0, 0.4)",
          }),
        }}
        classList={{
          "scale-x-75": props.loading,
          "bg-primary hover:bg-accent-hover": isPrimary() && !isDisabled(),
          "backdrop-blur-md bg-black-black":
            mergedProps.variant === "transparent",
          // Size
          "py-4 px-8": mergedProps.size === "large",
          "py-3 px-5": mergedProps.size === "medium",
          "py-2 px-4": mergedProps.size === "small",
          // Cursor
          "cursor-pointer": !props.loading,
          uppercase: mergedProps.uppercase,
          "bg-black-black text-black-gray": isDisabled() && !isGlow(),
          "border-1 bg-black-black": isOutline(),
          "border-white hover:border-accent-hover hover:text-accent-hover":
            isOutline() && !isDisabled(),
          "border-1 hover:border-white border-black-semiblack":
            isSecondary() && !isDisabled(),
          "border-1 border-black-semiblack":
            (isSecondary() && isDisabled()) || isOutline(),
          "text-black-semiblack": isDisabled() && isOutline(),
          "text-white": !isDisabled(),
          "flex-row-reverse": props.iconRight,
          "shadow-md shadow-primary bg-primary hover:shadow-lg hover:bg-accent-hover":
            isGlow() && !isDisabled(),
          "bg-black-gray text-shade-0": isGlow() && isDisabled(),
        }}
        {...props}
      >
        {!props.loading && c()}
      </button>
      <Spinner class="absolute left-1/2 -translate-x-1/2" />
    </div>
  );
}

export { Button };
