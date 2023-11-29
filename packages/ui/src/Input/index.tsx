import { Show, JSX, splitProps } from "solid-js";

interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {
  error?: string | boolean;
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

  let inputBaseClasses = `h-full w-full box-border py-2 rounded-md placeholder:text-darkSlate-400 outline-none focus-viible:outline-none ${
    local.inputClass || ""
  }`;
  if (local.icon) {
    inputBaseClasses += ` ${
      local.inputColor ? local.inputColor : "bg-darkSlate-700"
    } focus-visible:outline-none focus-visible:border-0`;
  } else {
    inputBaseClasses += ` px-4 focus-visible:outline-darkSlate-500`;
  }

  let errorClasses = local.error
    ? "border-2 border-solid border-red-500"
    : "border-0 border-transparent hover:border-darkSlate-500 active:border-darkSlate-500";
  let disabledClasses = local.disabled ? "text-darkSlate-300" : "text-white";
  let inputColorClasses = local.inputColor
    ? local.inputColor
    : "bg-darkSlate-600";

  let containerClasses = `h-10 gap-2 box-border transition-all duration-100 rounded-md ease-in-out ${
    local.class || ""
  }`;
  if (local.icon) {
    containerClasses += ` ${
      local.inputColor ? local.inputColor : "bg-darkSlate-700"
    } flex items-center px-4`;
  }

  return (
    <>
      <div class={containerClasses}>
        <input
          class={`${inputBaseClasses} ${disabledClasses} ${errorClasses} ${inputColorClasses}`}
          disabled={local.disabled}
          {...otherProps}
        />
        <Show when={local.icon}>
          <span class="text-darkSlate-300">{local.icon}</span>
        </Show>
      </div>

      <Show when={local.error}>
        <div class="text-red-500 text-left mt-2 font-light">{local.error}</div>
      </Show>
    </>
  );
}

export { Input };
