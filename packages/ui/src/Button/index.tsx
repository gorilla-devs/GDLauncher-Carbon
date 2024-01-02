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
type Type = "primary" | "secondary" | "glow" | "outline" | "transparent";

interface Props
  extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "type"> {
  children: HTMLElement | string | JSX.Element;
  style?: any;
  textColor?: string;
  type?: Type;
  rounded?: boolean;
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  uppercase?: boolean;
  loading?: boolean;
  size?: Size;
  percentage?: number;
  variant?: string;
  cursor?: string;
}

const getVariant = (
  type: Type,
  rounded: boolean,
  size: Size,
  isDisabled: boolean,
  uppercase: boolean,
  iconRight: boolean,
  isLoading: boolean,
  variant: string,
  cursor: string | undefined,
  textColor?: string
) => {
  const isLarge = size === "large";
  const isMedium = size === "medium";
  const isSmall = size === "small";

  const commonStyle = {
    ...(textColor && { [textColor]: true }),
    "transition-all": true,
    "overflow-hidden": true,
    "duration-100": true,
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
    "cursor-pointer": !isLoading && !isDisabled && !cursor,
    "cursor-not-allowed": isDisabled && !cursor,
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
      [`${!isDisabled ? `bg-${variant}-500` : "bg-[#1D2028]"}`]: true,
      [`${!isDisabled ? `hover:bg-${variant}-300` : ""}`]: true,
      // "filter brightness-75": isDisabled,
      "text-[#404759]": isDisabled,
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
      "border-1": true,
      "text-white": !isDisabled,
      "text-darkSlate-500": isDisabled,
      "border-white": !isDisabled,
      "border-darkSlate-500": isDisabled,
      "hover:border-primary-300": !isDisabled,
      "hover:text-primary-300": !isDisabled,
      "bg-transparent": !isDisabled,
      "bg-darkSlate-700": isDisabled,
    },
    glow: {
      ...commonStyle,
      [`bg-${variant}-500`]: !isDisabled,
      [`drop-shadow-[0_0px_12px_var(--${variant}-500)]`]: !isDisabled,
      "bg-[#404759]": isDisabled,
      "text-[#8A8B8F]": isDisabled,
      "border-0": true,
    },
    transparent: {
      ...commonStyle,
      // "backdrop-blur-md": true,
      "bg-darkSlate-800": true,
      "text-darkSlate-500": isDisabled,
      "border-0": true,
    },
  };

  return variants[type];
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
      type: "primary",
      size: "large",
      uppercase: false,
      iconRight: false,
      rounded: false,
    },
    props
  );

  return (
    <button
      classList={{
        ...getVariant(
          props.type || "primary",
          mergedProps.rounded,
          props.size || "medium",
          !!props.disabled,
          mergedProps.uppercase,
          !!props.iconRight,
          !!props.loading,
          props.variant || "primary",
          props.cursor,
          props.textColor
        ),
        ...props.classList,
      }}
      {...(others as JSX.ButtonHTMLAttributes<HTMLButtonElement>)}
      style={{
        ...(mergedProps.type === "transparent" && {
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
