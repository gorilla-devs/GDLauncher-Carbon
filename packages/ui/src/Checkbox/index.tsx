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

  return (
    <div class="flex items-center gap-2 font-sans">
      <div
        class="flex justify-center items-center h-5 w-5 min-w-5 min-h-5 rounded-md hover:border-lightGray hover:border-1 box-border cursor-pointer"
        classList={{
          "bg-primary-500": checked() && !props.disabled,
          "bg-darkSlate-500": !checked(),
          "bg-darkSlate-900": props.disabled,
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
          <div class="h-7 w-7 min-w-7 min-h-7 rounded-md -z-10 opacity-20 bg-light-300" />
        </Show>
      </div>
      <Show when={props.children}>{props.children}</Show>
    </div>
  );
}

export { Checkbox };
