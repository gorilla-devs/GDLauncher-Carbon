import { createSignal, Show } from "solid-js";

export interface Props {
  checked?: boolean;
  disabled?: boolean;
  onChange?: (_checked: boolean) => void;
}

function Checkbox(props: Props) {
  const isChecked = () => props.checked;
  const [checked, setChecked] = createSignal(isChecked());

  return (
    <div
      class="flex justify-center items-center h-5 w-5 rounded-md hover:border-lightGray hover:border-1 box-border"
      classList={{
        "bg-primary-500": checked(),
        "bg-darkSlate-500": !checked(),
        "bg-darkSlate-800": props.disabled,
      }}
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
