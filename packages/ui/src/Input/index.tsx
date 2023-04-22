import { Show, JSX, splitProps } from "solid-js";

export interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  inputColor?: string;
  icon?: Element | any;
}

function Input(props: Props) {
  const [local, otherProps] = splitProps(props, [
    "class",
    "icon",
    "inputClass",
    "disabled",
    "error",
    "inputColor",
  ]);

  return (
    <>
      <div
        class={`h-10 gap-2 box-border transition-all duration-100 ease-in-out ${
          local.class || ""
        }`}
        classList={{
          "bg-darkSlate-700 rounded-full px-4 flex items-center max-w-max":
            local.icon,
          "rounded-md": !local.icon,
        }}
      >
        <Show when={local.icon}>
          <span class="text-darkSlate-500">{local.icon}</span>
        </Show>
        <input
          class={`border-0 border-transparent h-full w-full box-border py-2 rounded-md placeholder:text-darkSlate-500 ${
            local.inputClass || ""
          } ${local.inputColor}
           outline-none focus-viible:outline-none
          `}
          classList={{
            "bg-darkSlate-700 focus-visible:outline-none focus-visible:border-0":
              local.icon,
            "px-4 focus-visible:outline-darkSlate-500 hover:border-darkSlate-500 active:border-darkSlate-500":
              !local.icon,
            "text-darkSlate-700": local.disabled,
            "text-white": !local.disabled,
            "border-red-500": !!local.error,
            "bg-darkSlate-600": !local.inputColor,
          }}
          {...otherProps}
        />
      </div>

      <Show when={local.error}>
        <div class="text-red-500 text-left">{local.error}</div>
      </Show>
    </>
  );
}

export { Input };
