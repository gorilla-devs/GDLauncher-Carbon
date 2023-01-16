import { createSignal, Show } from "solid-js";

export interface Props {
  checked: boolean;
  disabled: boolean;
  /* eslint-disable no-unused-vars */
  onChange?: (checked: boolean) => void;
}

// TODO: Add animation on click
function Checkbox(props: Props) {
  const isChecked = () => props.checked;
  const [checked, setChecked] = createSignal(isChecked());

  return (
    <div
      class={`flex justify-center items-center ${
        checked() ? "bg-accent-main" : "bg-shade-5"
      }  h-5 w-5 rounded-md hover:border-lightGray hover:border-1 box-border ${
        props.disabled ? "bg-shade-8" : ""
      }`}
      onClick={() => {
        if (!props.disabled) {
          setChecked(!checked());
          props.onChange?.(!checked());
        }
      }}
    >
      <Show when={checked()}>
        <div class="i-ri:check-line text-white animate-bounce-scale" />
      </Show>
    </div>
  );
}

export { Checkbox };
