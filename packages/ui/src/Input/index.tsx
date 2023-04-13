import { Show } from "solid-js";

export interface Props {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  inputColor?: string;
  icon?: Element | any;
  onInput?: (_e: InputEvent) => void;
}

function Input(props: Props) {
  return (
    <>
      <div
        class={`h-10 gap-2 box-border transition-all duration-100 ease-in-out ${
          props.class || ""
        }`}
        classList={{
          "bg-darkSlate-700 rounded-full px-4 flex items-center max-w-max":
            props.icon,
          "rounded-md": !props.icon,
        }}
      >
        <Show when={props.icon}>
          <span class="text-darkSlate-500">{props.icon}</span>
        </Show>
        <input
          class={`border-0 border-transparent h-full w-full box-border py-2 rounded-md placeholder:text-darkSlate-500 ${
            props.inputClass || ""
          } ${props.inputColor}
           outline-none focus-viible:outline-none
          `}
          classList={{
            "bg-darkSlate-700 focus-visible:outline-none focus-visible:border-0":
              props.icon,
            "px-4 focus-visible:outline-darkSlate-500 hover:border-darkSlate-500 active:border-darkSlate-500":
              !props.icon,
            "text-darkSlate-700": props.disabled,
            "text-white": !props.disabled,
            "border-red-500": !!props.error,
            "bg-darkSlate-600": !props.inputColor,
          }}
          placeholder={props.placeholder}
          value={props.value || ""}
          onInput={(e: InputEvent) => props?.onInput?.(e)}
        />
      </div>

      <Show when={props.error}>
        <div class="text-red-500 text-left">{props.error}</div>
      </Show>
    </>
  );
}

export { Input };
