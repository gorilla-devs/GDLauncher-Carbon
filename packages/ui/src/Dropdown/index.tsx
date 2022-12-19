import { createSignal, For, Show } from "solid-js";
import Style from "./Dropdown.module.scss";

interface Option {
  label: string;
  key: string;
}

interface Props {
  options: Option[];
  value: string;
  // eslint-disable-next-line no-unused-vars
  onChange: (option: Option) => void;
}

function Dropdown(props: Props) {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label || "";

  const [selectedValue, setSelectedValue] = createSignal(defaultValue());
  const [menuOpened, setMenuOpened] = createSignal(false);

  const toggleMenu = () => {
    setMenuOpened(true);
    setTimeout(() => {
      setMenuOpened(false);
    }, 100);
  };

  return (
    <div class="inline-block relative">
      <button
        class="flex justify-between bg-black-semiblack text-black-lightGray font-semibold py-2 px-4 rounded-full inline-flex items-center border-0 min-w-32"
        onMouseEnter={() => {
          setMenuOpened(true);
        }}
      >
        <span class="">{selectedValue()}</span>
        <span class="i-ri:arrow-drop-down-line text-3xl" />
      </button>
      <ul
        class={`absolute text-black-lightGray pt-1 ${
          menuOpened() ? "block" : "hidden"
        } list-none m-0 p-0 w-32`}
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
