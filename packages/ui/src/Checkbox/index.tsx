import { createEffect, createSignal, Show } from "solid-js";

interface Props {
  checked?: boolean;
  indeterminate?: boolean;
  disabled?: boolean;
  onChange?: (_checked: boolean) => void;
  children?: any;
}

function Checkbox(props: Props) {
  const isChecked = () => props.checked;
  const [checked, setChecked] = createSignal(isChecked());
  const [isHovered, setIsHovered] = createSignal(false); // Track hover state

  createEffect(() => {
    setChecked(props.checked);
  });

  const getBackgroundColor = () => {
    if (!checked() && props.indeterminate) {
      return "bg-light-300/20";
    } else if (checked() && !props.disabled) {
      return "bg-primary-500";
    } else if (!checked()) {
      return "bg-darkSlate-500";
    }
    return "bg-darkSlate-900";
  };

  return (
    <div
      class="flex rounded-md items-center gap-2 cursor-pointer"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={() => {
        if (!props.disabled) {
          const check = !checked();
          setChecked(check);
          props.onChange?.(check);
        }
      }}
    >
      <div
        class={`flex justify-center items-center h-5 w-5 min-w-5 min-h-5 rounded-md box-border ${
          isHovered() && !props.disabled
            ? "border-darkSlate-300 border-solid border"
            : ""
        } ${getBackgroundColor()}`}
      >
        <Show when={checked()}>
          <div
            class="i-ri:check-line text-lightSlate-50 animate-bounce-scale"
            classList={{
              "text-lightSlate-50": !props.disabled,
              "text-darkSlate-500": props.disabled,
            }}
          />
        </Show>
        <Show when={!checked() && props.indeterminate}>
          <div class="h-3 w-3 min-w-3 min-h-3 rounded-sm bg-primary-500" />
        </Show>
      </div>
      <Show when={props.children}>{props.children}</Show>
    </div>
  );
}

export { Checkbox };
