import { mergeProps, Show } from "solid-js";

interface Props {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  class?: string;
  icon?: Element | any;
  onInput?: () => void;
}

function Input(props: Props) {
  const mergedProps = mergeProps({ props: "" }, props);

  return (
    <div class="h-10">
      <div
        class="bg-black-black flex items-center gap-2 max-w-max h-full"
        classList={{
          "bg-black-semiblack rounded-full px-4": props.icon,
          "rounded-md": !props.icon,
        }}
      >
        <span class="text-black-gray">{props.icon}</span>
        <input
          class={`bg-black-black border-1 border-transparent h-full box-border py-2 rounded-md placeholder:text-black-gray ${
            props.class || ""
          }`}
          style={{
            outline: "none",
          }}
          classList={{
            "border-status-red": !!mergedProps.error,
            "border-transparent": !mergedProps.error,
            "text-black-semiblack": props.disabled,
            "bg-black-semiblack focus-visible:outline-none focus-visible:border-0":
              props.icon,
            "text-white": !props.disabled,
            "px-4 focus-visible:outline-black-gray withIcon": !props.icon,
            "hover:border-black-gray active:border-black-gray": !props.icon,
          }}
          placeholder={props.placeholder}
          value={props.value || ""}
          onInput={props.onInput}
        />
      </div>

      <Show when={props.error}>
        <div class="text-status-red text-left">{props.error}</div>
      </Show>
    </div>
  );
}

export { Input };
