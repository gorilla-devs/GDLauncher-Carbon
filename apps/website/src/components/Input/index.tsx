import { Show } from "solid-js";

type Props = {
  class?: string;
  type?: string;
  placeholder?: string;
  label?: any;
  onClick?: () => void;
};

function Input(props: Props) {
  return (
    <div class="flex flex-col gap-4">
      <Show when={props.label}>
        <label for={props.label}>{props.label}</label>
      </Show>
      <input class="rounded-2xl text-black py-4 px-6" id={props.label || ""} type={props.type || ''} placeholder={props.placeholder || ''}/>
    </div>
  );
}

export default Input;
