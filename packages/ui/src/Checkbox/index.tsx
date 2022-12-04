import { createSignal, Show } from "solid-js";

interface Props {
  checked: boolean;
  /* eslint-disable no-unused-vars */
  onChange?: (checked: boolean) => void;
}

function Checkbox(props: Props) {
  const [checked, setChecked] = createSignal(false);

  return (
    <div
      class={`flex justify-center items-center ${
        checked() ? "bg-accent-main" : "bg-black-gray"
      } h-5 w-5 rounded-md`}
      onClick={() => {
        setChecked(!checked());
        props.onChange?.(!checked());
      }}
    >
      <Show when={checked()}>
        <div class="i-ri:check-line text-white animate-bouncescale" />
      </Show>
    </div>
  );
}

export { Checkbox };
