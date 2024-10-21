import { JSX, Show, createEffect, createSignal } from "solid-js";
import { cva, type VariantProps } from "class-variance-authority";

const switchStyles = cva(
  "absolute top-0 left-0 right-0 bottom-0 transition-colors duration-100 ease-in-out rounded-full",
  {
    variants: {
      state: {
        default: "bg-darkSlate-500",
        checked: "bg-primary-500",
        disabled: "bg-darkSlate-700",
        disabledChecked: "bg-primary-700",
        indeterminate: "bg-darkSlate-500",
        loading: "bg-darkSlate-500",
      },
      cursor: {
        pointer: "cursor-pointer",
        notAllowed: "cursor-not-allowed",
      },
    },
    defaultVariants: {
      state: "default",
      cursor: "pointer",
    },
  }
);

const toggleStyles = cva(
  "absolute content-[] w-4 h-4 bottom-0.5 rounded-full transition-all duration-100 ease-in-out",
  {
    variants: {
      position: {
        start: "translate-x-0.5 bg-darkSlate-800",
        middle: "translate-x-3 bg-white",
        end: "translate-x-5 bg-white",
      },
      disabled: {
        true: "bg-darkSlate-50",
        false: "",
      },
    },
    defaultVariants: {
      position: "start",
      disabled: false,
    },
  }
);

interface Props
  extends JSX.InputHTMLAttributes<HTMLInputElement>,
    VariantProps<typeof switchStyles> {
  isIndeterminate?: boolean;
  isLoading?: boolean;
}

function Switch(props: Props) {
  const [state, setState] = createSignal<{
    togglePosition: "start" | "middle" | "end";
    switchState:
      | "default"
      | "checked"
      | "disabled"
      | "disabledChecked"
      | "indeterminate"
      | "loading";
    cursorStyle: "pointer" | "notAllowed";
    isLoading: boolean;
  }>({
    togglePosition: "start",
    switchState: "default",
    cursorStyle: "pointer",
    isLoading: false,
  });

  createEffect(() => {
    if (props.isLoading) {
      setState({
        togglePosition: "middle" as const,
        switchState: "loading" as const,
        cursorStyle: "notAllowed" as const,
        isLoading: true,
      });
    } else if (props.isIndeterminate) {
      setState({
        togglePosition: "middle",
        switchState: "indeterminate",
        cursorStyle: "pointer",
        isLoading: false,
      });
    } else if (props.checked) {
      setState({
        togglePosition: "end",
        switchState: props.disabled ? "disabledChecked" : "checked",
        cursorStyle: props.disabled ? "notAllowed" : "pointer",
        isLoading: false,
      });
    } else {
      setState({
        togglePosition: "start",
        switchState: props.disabled ? "disabled" : "default",
        cursorStyle: props.disabled ? "notAllowed" : "pointer",
        isLoading: false,
      });
    }
  });

  return (
    <label class="relative inline-block w-10 h-5 group">
      <input
        {...props}
        class="opacity-0 w-0 h-0 peer"
        type="checkbox"
        disabled={props.disabled}
      />
      <span
        class={switchStyles({
          state: state().switchState,
          cursor: state().cursorStyle,
        })}
      >
        {state().isLoading && (
          <i
            class="i-ri:loader-4-line absolute text-xs text-darkSlate-50 z-50 animate-spin"
            style={{ left: "calc(50% - 0.38rem)", top: "calc(50% - 0.38rem)" }}
          />
        )}
        <span
          class={toggleStyles({
            position: state().togglePosition,
            disabled: props.disabled,
          })}
        />
      </span>
      <Show when={!props.disabled}>
        <span class="absolute inset-0 rounded-full transition-all duration-200 ease-in-out group-hover:ring-1 group-hover:ring-darkSlate-200" />
      </Show>
    </label>
  );
}

export { Switch };
