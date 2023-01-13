import style from "./Switch.module.scss";

export interface Props {
  checked?: boolean;
  disabled?: boolean;
  /* eslint-disable no-unused-vars */
  onChange?: (e: Event) => void;
}

function Switch(props: Props) {
  return (
    <div class="flex items-center">
      <input
        type="checkbox"
        id="isActive"
        class={`hidden ${props.disabled ? style.disabled : ""} `}
        checked={!props.checked}
        onChange={(e) => {
          if (!props.disabled) props?.onChange?.(e);
        }}
      />
      <label
        for="isActive"
        class={`${style.slider} relative mr-4 w-12 h-6 rounded-full ${
          props.disabled ? "bg-shade-8" : "bg-accent-main"
        } cursor-pointer`}
      />
    </div>
  );
}

export { Switch };
