import { children, mergeProps, Show, JSX, splitProps } from "solid-js";
import { Spinner } from "../Spinner";

type Size = "small" | "medium" | "large";
type Variant = "primary" | "secondary" | "glow" | "outline" | "transparent";

export interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  children: HTMLElement | string;
  class?: string;
  textColor?: string;
  variant?: Variant;
  rounded?: boolean;
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
  rounded: boolean,
  size: Size,
  isDisabled: boolean,
  uppercase: boolean,
  iconRight: boolean,
  isLoading: boolean,
  textColor?: string
) => {
  const isLarge = size === "large";
  const isMedium = size === "medium";
  const isSmall = size === "small";

  const commonStyle = {
    ...(textColor && { [textColor]: true }),
    "transition-all": true,
    "duration-300": true,
    "ease-in-out": true,
    "font-main": true,
    "max-w-max": !isLoading,
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
    "rounded-full": rounded,
    "rounded-md": !rounded,
    uppercase,
    "cursor-pointer": !isLoading,
    "box-border": true,
    "border-solid": true,
    "scale-x-100": isLoading,
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
      "border-0": true,
    },
    secondary: {
      ...commonStyle,
      "border-1": true,
      "hover:border-white": !isDisabled,
      "border-shade-7": !isDisabled,
      "bg-shade-8": !isDisabled,
      "bg-shade-5": isDisabled,
      "text-shade-5": isDisabled,
    },
    outline: {
      ...commonStyle,
      "border-2": true,
      "text-shade-7": !isDisabled,
      "text-shade-5": isDisabled,
      "border-white": !isDisabled,
      "border-shade-5": isDisabled,
      "hover:border-primary-hover": !isDisabled,
      "hover:text-primary-hover": !isDisabled,
      "bg-transparent": !isDisabled,
      "bg-shade-7": isDisabled,
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
      "border-0": true,
    },
    transparent: {
      ...commonStyle,
      "backdrop-blur-md": true,
      "bg-shade-8": true,
      "text-shade-5": isDisabled,
      "border-0": true,
    },
  };

  return variants[variant];
};

function Button(props: Props) {
  const c = children(() => props.children);

  const [_, others] = splitProps(props, [
    "icon",
    "iconRight",
    "uppercase",
    "loading",
    "size",
    "children",
  ]);

  const mergedProps = mergeProps(
    {
      variant: "primary",
      size: "large",
      uppercase: false,
      iconRight: false,
      rounded: true,
    },
    props
  );

  return (
    <div class="w-fit h-fit">
      <button
        classList={getVariant(
          props.variant || "primary",
          mergedProps.rounded,
          props.size || "medium",
          !!props.disabled,
          mergedProps.uppercase,
          !!props.iconRight,
          !!props.loading,
          props.textColor
        )}
        {...(others as JSX.ButtonHTMLAttributes<HTMLButtonElement>)}
        style={{
          ...(mergedProps.variant === "transparent" && {
            background: "rgba(0, 0, 0, 0.4)",
          }),
        }}
      >
        <Show when={props.icon}>{props.icon}</Show>
        <Show
          when={!props.loading}
          fallback={
            <div class="w-12 h-12 flex justify-center items-center">
              <Spinner />
            </div>
          }
        >
          {c()}
        </Show>
      </button>
    </div>
  );
}

export { Button };
