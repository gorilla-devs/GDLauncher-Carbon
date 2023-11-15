import { createEffect, createSignal, Show } from "solid-js";

interface Props {
  checked?: boolean;
  indeterminate?: boolean;
  disabled?: boolean;
  onChange?: (_checked: boolean) => void;
  children?: any;
}

function Checkbox(props: Props) {
  const isChecked = () => props.checked;

  // eslint-disable-next-line solid/reactivity
  const [checked, setChecked] = createSignal(isChecked());

  createEffect(() => {
    setChecked(props.checked);
  });

  const getColor = () => {
    if (checked() && !props.disabled) {
      return "bg-primary-500";
    } else if (!checked()) {
      return "bg-darkSlate-500";
    }
    return "bg-darkSlate-900";
  };

  return (
    <div class="flex justify-center items-center h-5 w-5 min-w-5 min-h-5 rounded-md gap-2 font-sans ">
      <div
        class={`flex justify-center items-center  ${
          !checked() || (checked() && !props.indeterminate)
            ? "h-5 w-5 min-w-5 min-h-5 rounded-md"
            : "rounded-sm"
        } hover:border-lightGray hover:border-1 box-border cursor-pointer ${getColor()}`}
        classList={{
          // "bg-primary-500": checked() && !props.disabled,
          // "bg-darkSlate-500": !checked(),
          // "bg-darkSlate-900": props.disabled,
          "h-3 w-3 min-w-3 min-h-3": props.indeterminate && checked(),
        }}
        onClick={() => {
          if (!props.disabled) {
            const check = setChecked(!checked());
            props.onChange?.(check);
          }
        }}
      >
        <Show when={checked() && !props.indeterminate}>
          <div
            class="i-ri:check-line text-white animate-bounce-scale"
            classList={{
              "text-white": !props.disabled,
              "text-darkSlate-500": props.disabled,
            }}
          />
        </Show>
        <Show when={checked() && props.indeterminate}>
          <div class="h-5 w-5 min-w-5  min-h-5 rounded-md  opacity-20 bg-light-300" />
        </Show>
      </div>
      <Show when={props.children}>{props.children}</Show>
    </div>
  );
}

export { Checkbox };
