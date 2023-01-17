import { createSignal, For, Show } from "solid-js";

export interface Option {
  label: string;
  key: string;
}

export interface Props {
  options: Option[];
  value: string;
  error?: boolean;
  disabled?: boolean;
  rounded?: boolean;
  label?: string;
  // eslint-disable-next-line no-unused-vars
  onChange?: (option: Option) => void;
}

function Dropdown(props: Props) {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label ||
    props.options[0]?.label;

  const [selectedValue, setSelectedValue] = createSignal(defaultValue());
  const [menuOpened, setMenuOpened] = createSignal(false);

  const toggleMenu = () => {
    if (props.disabled) return;
    setMenuOpened(true);
    setTimeout(() => {
      setMenuOpened(false);
    }, 100);
  };

  return (
    <div class="inline-block relative">
      <Show when={!props.rounded && props.label}>
        <p
          class="mt-0 mb-2 font-bold"
          classList={{
            "text-white": !props.disabled,
            "text-shade-0": props.disabled,
          }}
        >
          {props.label}
        </p>
      </Show>
      <button
        class="group flex justify-between bg-shade-7 font-semibold py-2 px-4 inline-flex items-center min-w-45 min-h-10 box-border"
        onClick={() => {
          if (props.disabled) return;
          setMenuOpened(!menuOpened());
        }}
        onBlur={() => {
          console.log("TEST");
          // setMenuOpened(false);
        }}
        classList={{
          "border-0": !props.error,
          "border-1 border-status-red": props.error,
          "text-shade-0 hover:text-white": !props.disabled && !props.error,
          "text-shade-5": props.error,
          "rounded-full": props.rounded,
          rounded: !props.rounded,
        }}
      >
        <span
          classList={{
            "text-white": props.error,
            "text-shade-0 hover:text-white group-hover:text-white":
              !props.disabled && !props.error,
            "text-shade-5": props.disabled,
          }}
        >
          {selectedValue()}
        </span>
        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-shade-0 group-hover:text-white":
              !props.disabled && !props.error,
            "text-white": props.error,
            "text-shade-5": props.disabled,
          }}
        />
      </button>
      <ul
        class={`absolute text-shade-0 pt-1 z-20 ${
          menuOpened() ? "block" : "hidden"
        } list-none m-0 p-0 w-45`}
      >
        <For each={props.options}>
          {(option) => (
            <li
              class="first:rounded-t last:rounded-b bg-shade-7 hover:bg-[#343946] py-2 px-4 block whitespace-no-wrap text-shade-0 no-underline"
              onClick={() => {
                console.log("OPTIOn", option.label);
                setSelectedValue(option.label);
                props.onChange?.(option);
                toggleMenu();
              }}
            >
              {option.label}
            </li>
          )}
        </For>
      </ul>
    </div>
  );
}

export { Dropdown };
