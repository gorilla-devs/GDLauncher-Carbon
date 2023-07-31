import { createEffect, createSignal, Show } from "solid-js";

interface Props {
  checked?: boolean;
  disabled?: boolean;
  onChange?: (_checked: boolean) => void;
}

function Checkbox(props: Props) {
  const isChecked = () => props.checked;
  // eslint-disable-next-line solid/reactivity
  const [checked, setChecked] = createSignal(isChecked());

  createEffect(() => {
    if (!props.disabled) setChecked(props.checked);
  });

  return (
    <div
      class="flex justify-center items-center h-5 w-5 rounded-md hover:border-lightGray hover:border-1 box-border cursor-pointer"
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
      <Show when={checked()}>
        <div
          class="i-ri:check-line text-white animate-bounce-scale"
          classList={{
            "text-white": !props.disabled,
            "text-darkSlate-500": props.disabled,
          }}
        />
      </Show>
    </div>
  );
}

export { Checkbox };
