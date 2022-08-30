import { Show } from "solid-js";

type Props = {
  class?: string;
  label?: any;
  onClick?: () => void;
};

function Input(props: Props) {
  return (
    <div class="flex flex-col gap-4">
      <Show when={props.label}>
        <label for={props.label}>{props.label}</label>
      </Show>
      <input class="rounded-2xl text-black py-2 px-4" id={props.label || ""} />
    </div>
  );
}

export default Input;
