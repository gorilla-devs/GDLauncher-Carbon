import { JSX, createEffect, createSignal } from "solid-js";

interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {
  isIndeterminate?: boolean;
  isLoading?: boolean;
  disabled?: boolean;
}

function Switch(props: Props) {
  const [state, setState] = createSignal({
    togglePosition: "translate-x-0.5 bg-white",
    bgColor: "bg-darkSlate-500",
    cursorStyle: "cursor-pointer",
    loadingClass: "",
  });

  createEffect(() => {
    if (props.isLoading) {
      setState({
        togglePosition: "translate-x-3 bg-darkSlate-800",
        bgColor: "bg-darkSlate-500",
        cursorStyle: "cursor-not-allowed",
        loadingClass: "relative",
      });
    } else if (props.isIndeterminate) {
      setState({
        togglePosition: "translate-x-3 bg-white",
        bgColor: "bg-darkSlate-500",
        cursorStyle: "cursor-pointer",
        loadingClass: "",
      });
    } else if (props.checked) {
      setState({
        togglePosition: "translate-x-5 bg-white",
        bgColor: props.disabled ? "bg-primary-700" : "bg-primary-500",
        cursorStyle: props.disabled ? "cursor-not-allowed" : "cursor-pointer",
        loadingClass: "",
      });
    } else {
      setState({
        togglePosition: "translate-x-0.5 bg-darkSlate-800",
        bgColor: props.disabled ? "bg-darkSlate-700" : "bg-darkSlate-500",
        cursorStyle: props.disabled ? "cursor-not-allowed" : "cursor-pointer",
        loadingClass: "",
      });
    }
  });

  return (
    <label class={`relative inline-block w-10 h-5 ${state().loadingClass}`}>
      <input
        {...props}
        class="opacity-0 w-0 h-0 peer"
        type="checkbox"
        disabled={props.disabled}
      />
      <span
        class={`absolute ${
          state().cursorStyle
        } top-0 left-0 right-0 bottom-0 transition-colors duration-100 ease-in-out rounded-full ${
          state().bgColor
        }`}
      >
        {props.isLoading && (
          <i
            class="i-ri:loader-4-line absolute text-xs text-darkSlate-50 z-50 animate-spin"
            style={{ left: "calc(50% - 0.38rem)", top: "calc(50% - 0.38rem)" }}
          />
        )}
        <span
          class={`absolute content-[] w-4 h-4 bottom-0.5 rounded-full transition-all duration-100 ease-in-out peer-disabled:bg-darkSlate-50 ${
            state().togglePosition
          }`}
        />
      </span>
    </label>
  );
}

export { Switch };
