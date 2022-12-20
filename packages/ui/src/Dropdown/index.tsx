import { createSignal, For } from "solid-js";

interface Option {
  label: string;
  key: string;
}

interface Props {
  options: Option[];
  value: string;
  error?: boolean;
  disabled?: boolean;
  // eslint-disable-next-line no-unused-vars
  onChange: (option: Option) => void;
}

function Dropdown(props: Props) {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label || "";

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
      <button
        class="group flex justify-between bg-black-semiblack font-semibold py-2 px-4 rounded-full inline-flex items-center min-w-45 min-h-10 box-border"
        onClick={() => {
          if (props.disabled) return;
          setMenuOpened(true);
        }}
        onBlur={() => {
          setMenuOpened(false);
        }}
        classList={{
          "border-0": !props.error,
          "border-1 border-status-red": props.error,
          "text-black-lightGray hover:text-white": !props.disabled,
          "text-black-gray": props.error,
        }}
      >
        <span
          classList={{
            "text-white": props.error,
            "text-black-lightGray hover:text-white group-hover:text-white":
              !props.disabled,
            "text-black-gray": props.error,
          }}
        >
          {selectedValue()}
        </span>
        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-black-lightGray group-hover:text-white": !props.disabled,
            "text-black-gray": props.error,
          }}
        />
      </button>
      <ul
        class={`absolute text-black-lightGray pt-1 ${
          menuOpened() ? "block" : "hidden"
        } list-none m-0 p-0 w-45`}
      >
        <For each={props.options}>
          {(option) => (
            <li
              class="first:rounded-t last:rounded-b bg-black-semiblack hover:bg-[#343946] py-2 px-4 block whitespace-no-wrap text-black-lightGray no-underline"
              onClick={() => {
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
