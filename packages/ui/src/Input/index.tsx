import { Show } from "solid-js";

interface Props {
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
        class={`bg-black-black flex items-center gap-2 max-w-max h-full ${
          props.icon ? "bg-black-semiblack rounded-full px-4" : "rounded-md"
        } ${props.class || ""}`}
      >
        <Show when={props.icon}>
          <span class="text-black-gray">{props.icon}</span>
        </Show>
        <input
          class={`bg-black-black border-1 border-transparent h-full box-border py-2 rounded-md placeholder:text-black-gray ${
            props.inputClass || ""
          }
           ${props.error ? "border-status-red" : ""}
           ${props.disabled ? "text-black-semiblack" : "text-white"}
           ${
             props.icon
               ? "bg-black-semiblack focus-visible:outline-none focus-visible:border-0"
               : "px-4 focus-visible:outline-black-gray hover:border-black-gray active:border-black-gray"
           }
          
          `}
          style={{
            outline: "none",
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
