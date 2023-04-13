import {
  children,
  mergeProps,
  Show,
  JSX,
  splitProps,
  Switch,
  Match,
} from "solid-js";
import { Spinner } from "../Spinner";

type Size = "small" | "medium" | "large";
type Variant = "primary" | "secondary" | "glow" | "outline" | "transparent";

export interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  children: HTMLElement | string | JSX.Element;
  style?: any;
  textColor?: string;
  variant?: Variant;
  rounded?: boolean;
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  uppercase?: boolean;
  loading?: boolean;
  size?: Size;
  percentage?: number;
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
    "overflow-hidden": true,
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
      "hover:bg-accent": !isDisabled,
      "bg-darkSlate-800": isDisabled,
      "text-darkSlate-500": isDisabled,
      "border-0": true,
    },
    secondary: {
      ...commonStyle,
      "border-1": true,
      "hover:border-white": !isDisabled,
      "border-darkSlate-500": !isDisabled,
      "bg-darkSlate-700": true,
      "text-darkSlate-500": isDisabled,
      "cursor-not-allowed": isDisabled,
    },
    outline: {
      ...commonStyle,
      "border-2": true,
      "text-white": !isDisabled,
      "text-darkSlate-500": isDisabled,
      "border-white": !isDisabled,
      "border-darkSlate-500": isDisabled,
      "hover:border-primary-hover": !isDisabled,
      "hover:text-primary-hover": !isDisabled,
      "bg-transparent": !isDisabled,
      "bg-darkSlate-700": isDisabled,
    },
    glow: {
      ...commonStyle,
      "shadow-md": !isDisabled,
      "shadow-primary": !isDisabled,
      "bg-primary": !isDisabled,
      "hover:shadow-lg": !isDisabled,
      "hover:bg-primary-hover": !isDisabled,
      "bg-darkSlate-500": isDisabled,
      "text-darkSlate-50": isDisabled,
      "border-0": true,
    },
    transparent: {
      ...commonStyle,
      "backdrop-blur-md": true,
      "bg-darkSlate-800": true,
      "text-darkSlate-500": isDisabled,
      "border-0": true,
    },
  };

  return variants[variant];
};

const Loading = (props: {
  children: HTMLElement | string | JSX.Element;
  percentage: number | undefined;
}) => {
  return (
    <Switch>
      <Match when={props.percentage === undefined}>
        <div class="w-12 h-12 flex justify-center items-center">
          <Spinner />
        </div>
      </Match>
      <Match when={props.percentage !== undefined}>
        <div class="w-20 h-11 flex justify-center items-center relative">
          <div
            class="bg-green-500 text-xs leading-none py-1 absolute top-0 left-0 bottom-0"
            style={{ width: `${props.percentage}%` }}
          />
          <div>
            <span class="z-10 absolute left-1/2 -translate-x-1/2 top-1/2 -translate-y-1/2">
              {props.children}
            </span>
          </div>
        </div>
      </Match>
    </Switch>
  );
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
        ...props.style,
      }}
    >
      <Show when={props.icon}>{props.icon}</Show>
      <Show
        when={!props.loading}
        fallback={<Loading percentage={props.percentage}>{c()}</Loading>}
      >
        {c()}
      </Show>
    </button>
  );
}

export { Button };
