import { children, mergeProps, Show, JSX } from "solid-js";
import { Spinner } from "../Spinner";
// import "./Button.css";

type Size = "small" | "medium" | "large";
type Variant = "primary" | "secondary" | "glow" | "outline" | "transparent";

export interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  children: HTMLElement | string;
  class?: string;
  variant?: Variant;
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  uppercase?: boolean;
  loading?: boolean;
  size?: Size;
  onClick?: (_: MouseEvent) => void;
}

const getVariant = (
  variant: Variant,
  size: Size,
  isDisabled: boolean,
  uppercase: boolean,
  iconRight: boolean,
  isLoading: boolean
) => {
  const isLarge = size === "large";
  const isMedium = size === "medium";
  const isSmall = size === "small";

  const commonStyle = {
    transition: true,
    "duration-200": true,
    "ease-in-out": true,
    "font-main": true,
    "max-w-max": true,
    "font-bold": true,
    flex: true,
    "justify-center": true,
    "items-center": true,
    "gap-2": true,
    relative: true,
    "py-4 px-8": isLarge && !isLoading,
    "py-3 px-5": isMedium && !isLoading,
    "py-2 px-4": isSmall && !isLoading,
    "h-12": isLarge,
    "h-11": isMedium,
    "h-9": isSmall,
    "rounded-full": true,
    uppercase,
    "cursor-pointer": !isLoading,
    "box-border": true,
    "w-12": isLoading,
    "p-0": isLoading,
    "text-white": !isDisabled,
    "flex-row-reverse": iconRight,
  };

  const variants = {
    primary: {
      ...commonStyle,
      "bg-primary": true,
      "hover:bg-primary-hover": !isDisabled,
      "bg-shade-8": isDisabled,
      "text-shade-5": isDisabled,
    },
    secondary: {
      ...commonStyle,
      "border-1": true,
      "hover:border-white": !isDisabled,
      "border-shade-7": true,
      "bg-shade-8": isDisabled,
      "text-shade-5": isDisabled,
    },
    outline: {
      ...commonStyle,
      "border-1": true,
      "border-shade-7": true,
      "text-shade-7": isDisabled,
      "bg-shade-8": isDisabled,
      "text-shade-5": isDisabled,
      "border-white": !isDisabled,
      "hover:border-accent-hover": !isDisabled,
      "hover:text-primary-hover": !isDisabled,
    },
    glow: {
      ...commonStyle,
      "shadow-md": !isDisabled,
      "shadow-primary": !isDisabled,
      "bg-primary": !isDisabled,
      "hover:shadow-lg": !isDisabled,
      "hover:bg-primary-hover": !isDisabled,
      "bg-shade-5": isDisabled,
      "text-shade-0": isDisabled,
    },
    transparent: {
      ...commonStyle,
      "backdrop-blur-md": true,
      "bg-shade-8": true,
      "text-shade-5": isDisabled,
    },
  };

  console.log("TEST", variants[variant]);
  return variants[variant];
};

function Button(props: Props) {
  const c = children(() => props.children);

  const mergedProps = mergeProps(
    { variant: "primary", size: "large", uppercase: false, iconRight: false },
    props
  );

  return (
    <div
      class={props.class}
      classList={getVariant(
        props.variant || "primary",
        props.size || "medium",
        !!props.disabled,
        mergedProps.uppercase,
        !!props.iconRight,
        !!props.loading
      )}
    >
      <Show when={props.icon}>{props.icon}</Show>
      <Show when={props.loading} fallback={c()}>
        <Spinner class="absolute left-1/2 -translate-x-1/2" />
      </Show>
    </div>
  );
}

export { Button };
