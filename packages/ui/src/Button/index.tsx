import {
  children,
  mergeProps,
  Show,
  JSX,
  splitProps,
  Switch,
  Match
} from "solid-js"
import { Spinner } from "../Spinner"
import { Dynamic } from "solid-js/web"

type Size = "small" | "medium" | "large"
type Type =
  | "primary"
  | "secondary"
  | "glow"
  | "outline"
  | "transparent"
  | "text"

interface Props
  extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "type"> {
  as?: "button" | "a" | "span" | "div"
  children: HTMLElement | string | JSX.Element
  style?: JSX.CSSProperties
  textColor?: string
  backgroundColor?: string
  type?: Type
  rounded?: boolean
  disabled?: boolean
  icon?: JSX.Element
  iconRight?: boolean
  uppercase?: boolean
  loading?: boolean
  size?: Size
  percentage?: number
  variant?: string
  cursor?: string
  fullWidth?: boolean
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
  textColor?: string,
  backgroundColor?: string,
  fullWidth?: boolean
) => {
  const isLarge = size === "large"
  const isMedium = size === "medium"
  const isSmall = size === "small"

  const commonStyle = {
    ...(textColor && { [textColor]: true }),
    ...(backgroundColor && {
      [backgroundColor]: true,
      "hover:brightness-120": true
    }),
    "transition-all": true,
    "overflow-hidden": true,
    "duration-100": true,
    "ease-in-out": true,
    "font-main": true,
    "max-w-max": !isLoading && !fullWidth,
    "w-full": fullWidth,
    "font-bold": true,
    flex: true,
    "justify-center": true,
    "items-center": true,
    "gap-2": true,
    relative: true,
    "py-4 px-8": isLarge && !rounded,
    "py-3 px-5": isMedium && !rounded,
    "py-2 px-4": isSmall && !rounded,
    "p-4": isLarge && rounded,
    "p-3": isMedium && rounded,
    "p-2": isSmall && rounded,
    "h-12": isLarge,
    "h-11": isMedium,
    "h-9": isSmall,
    "rounded-full": rounded,
    "rounded-md": !rounded,
    uppercase,
    "cursor-not-allowed": isDisabled && !cursor,
    "box-border": true,
    "border-solid": true,
    "scale-x-100": isLoading,
    "p-0": isLoading,
    "text-lightSlate-50": !isDisabled,
    "flex-row-reverse": iconRight
  }

  const variants = {
    primary: {
      ...commonStyle,
      [`${
        !isDisabled && !backgroundColor ? `bg-${variant}-500` : "bg-[#1D2028]"
      }`]: true,
      [`${!isDisabled && !backgroundColor ? `hover:bg-${variant}-700` : ""}`]:
        true,
      // "filter brightness-75": isDisabled,
      "text-[#404759]": isDisabled,
      "border-0": true
    },
    secondary: {
      ...commonStyle,
      "border-2": true,
      "hover:border-darkSlate-500": !isDisabled,
      "border-darkSlate-600": !isDisabled,
      "bg-darkSlate-700": true,
      "text-lightSlate-700": isDisabled,
      "cursor-not-allowed": isDisabled
    },
    outline: {
      ...commonStyle,
      "border-1": true,
      "text-lightSlate-50": !isDisabled,
      "text-lightSlate-700": isDisabled,
      "border-white": !isDisabled,
      "border-darkSlate-500": isDisabled,
      "hover:border-primary-300": !isDisabled,
      "hover:text-primary-300": !isDisabled,
      "bg-transparent": !isDisabled,
      "bg-darkSlate-700": isDisabled
    },
    glow: {
      ...commonStyle,
      [`bg-${variant}-500`]: !isDisabled,
      [`drop-shadow-[0_0px_12px_rgb(var(--${variant}-500))]`]: !isDisabled,
      "bg-[#404759]": isDisabled,
      "text-[#8A8B8F]": isDisabled,
      "border-0": true
    },
    transparent: {
      ...commonStyle,
      // "backdrop-blur-md": true,
      "bg-darkSlate-800": true,
      "text-lightSlate-700": isDisabled,
      "border-1": true,
      "border-transparent": true,
      "hover:border-1": !isDisabled,
      "hover:border-white": !isDisabled
    },
    text: {
      ...commonStyle,
      "bg-transparent": true,
      "text-lightSlate-50": true,
      "hover:bg-darkSlate-900": true,
      "border-0": true
    }
  }

  return variants[type]
}

const Loading = (props: {
  children: HTMLElement | string | JSX.Element
  percentage: number | undefined
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
  )
}

function Button(props: Props) {
  const c = children(() => props.children)

  const [_, others] = splitProps(props, [
    "as",
    "icon",
    "iconRight",
    "uppercase",
    "loading",
    "size",
    "children"
  ])

  const mergedProps = mergeProps(
    {
      type: "primary",
      size: "large",
      uppercase: false,
      iconRight: false,
      rounded: false
    },
    props
  )

  const component = props.as || "button"

  return (
    <Dynamic
      component={component}
      {...(others as JSX.ButtonHTMLAttributes<HTMLButtonElement>)}
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
          props.textColor,
          props.backgroundColor,
          props.fullWidth
        ),
        ...props.classList
      }}
      style={{
        ...(mergedProps.type === "transparent" && {
          background: "rgba(0, 0, 0, 0.9)"
        }),
        ...props.style
      }}
    >
      <Show when={props.icon}>{props.icon}</Show>
      <Show
        when={!props.loading}
        fallback={<Loading percentage={props.percentage}>{c()}</Loading>}
      >
        {c()}
      </Show>
    </Dynamic>
  )
}

export { Button }
