import { mergeProps, Show } from "solid-js";
import "./index.tsx";

type Props = {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  onInput: (e: InputEvent) => void;
};

function Input(props: Props) {
  const mergedProps = mergeProps({ props: "" }, props);

  return (
    <div>
      <input
        class="bg-black-black border-1 border-transparent hover:border-black-gray h-10 box-border py-2 px-4 rounded-md"
        classList={{
          "border-status-red": !!mergedProps.error,
          "border-transparent": !mergedProps.error,
          "text-black-semiblack": props.disabled,
          "text-white": !props.disabled
        }}
        placeholder={props.placeholder}
        value={props.value}
        onInput={props.onInput}
      />

      <Show when={props.error}>
        <div class="text-status-red text-left">{props.error}</div>
      </Show>
    </div>
  );
}

export { Input };
