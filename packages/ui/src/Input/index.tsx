import { Show } from "solid-js";

export interface Props {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  icon?: Element | any;
  /* eslint-disable no-unused-vars */
  onInput?: (e: InputEvent) => void;
}

function Input(props: Props) {
  return (
    <div class="h-10">
      <div
        class={`bg-shade-5 flex items-center gap-2 max-w-max h-full box-border transition-all duration-100 ease-in-out ${
          props.class || ""
        }`}
        classList={{
          "bg-shade-7 rounded-full px-4": props.icon,
          "rounded-md": !props.icon,
        }}
      >
        <Show when={props.icon}>
          <span class="text-shade-5">{props.icon}</span>
        </Show>
        <input
          class={`bg-shade-5 border-1 border-transparent h-full w-full box-border py-2 rounded-md placeholder:text-shade-5 ${
            props.inputClass || ""
          }
           outline-transparent
          `}
          classList={{
            "bg-shade-7 focus-visible:outline-none focus-visible:border-0":
              props.icon,
            "px-4 focus-visible:outline-shade-5 hover:border-shade-5 active:border-shade-5":
              !props.icon,
            "text-shade-7": props.disabled,
            "text-white": !props.disabled,
            "border-status-red": !!props.error,
          }}
          placeholder={props.placeholder}
          value={props.value || ""}
          onInput={(e: InputEvent) => props?.onInput?.(e)}
        />
      </div>

      <Show when={props.error}>
        <div class="text-status-red text-left">{props.error}</div>
      </Show>
    </div>
  );
}

export { Input };
