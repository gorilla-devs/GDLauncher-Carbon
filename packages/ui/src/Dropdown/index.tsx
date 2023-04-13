import { createSignal, For, Show, JSX } from "solid-js";
import { Button } from "../Button";

export type Option = {
  label: string;
  key: string;
};

export type Props = {
  options: Option[];
  value: string;
  error?: boolean;
  disabled?: boolean;
  rounded?: boolean;
  label?: string;
  onChange?: (_option: Option) => void;
  class?: string;
  containerClass?: string;
  id?: string;
  bgColorClass?: string;
  btnDropdown?: boolean;
  icon?: JSX.Element;
  placeholder?: string;
};
export interface DropDownButtonProps {
  children: JSX.Element;
  options: Option[];
  value: string;
  error?: boolean;
  disabled?: boolean;
  rounded?: boolean;
  label?: string;
  onChange?: (_value: string) => void;
  class?: string;
  id?: string;
  bgColorClass?: string;
  btnDropdown?: boolean;
  icon?: JSX.Element;
  placeholder?: string;
}

const Dropdown = (props: Props) => {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label ||
    props.options[0]?.label;

  const placeholder = () => props.placeholder;

  const [selectedValue, setSelectedValue] = createSignal(
    placeholder() || defaultValue()
  );
  const [menuOpened, setMenuOpened] = createSignal(false);
  const [focusIn, setFocusIn] = createSignal(false);

  const toggleMenu = () => {
    if (props.disabled) return;
    setMenuOpened(true);
    setTimeout(() => {
      setMenuOpened(false);
    }, 100);
  };

  return (
    <div
      class={`inline-block relative ${props.containerClass || ""}`}
      id={props.id}
    >
      <Show when={!props.rounded && props.label}>
        <p
          class="mt-0 mb-2 font-bold"
          classList={{
            "text-white": !props.disabled,
            "text-darkSlate-50": props.disabled,
          }}
        >
          {props.label}
        </p>
      </Show>
      <button
        class={`group flex justify-between font-semibold py-2 px-4 inline-flex items-center min-h-10 box-border ${props.class} ${props.bgColorClass}`}
        onClick={() => {
          if (props.disabled) return;
          setMenuOpened(!menuOpened());
        }}
        onBlur={() => {
          if (!focusIn()) {
            setMenuOpened(false);
          }
        }}
        classList={{
          "border-0": !props.error,
          "border-1 border-status-red": props.error,
          "text-darkSlate-50 hover:text-white": !props.disabled && !props.error,
          "text-darkSlate-500": props.error,
          "rounded-full": props.rounded,
          rounded: !props.rounded,
          "bg-darkSlate-700": !props.bgColorClass,
          "rounded-md": !props.btnDropdown,
        }}
      >
        <Show when={!props.btnDropdown}>
          <Show when={props.icon}>
            <span class="mr-2">{props.icon}</span>
          </Show>
          <span
            classList={{
              "text-white": props.error,
              "text-darkSlate-50 hover:text-white group-hover:text-white":
                !props.disabled && !props.error,
              "text-darkSlate-500": props.disabled,
            }}
          >
            {selectedValue()}
          </span>
        </Show>
        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-darkSlate-50 group-hover:text-white":
              !props.disabled && !props.error && !props.btnDropdown,
            "text-white": props.error || props.btnDropdown,
            "text-darkSlate-500": props.disabled,
          }}
        />
      </button>

      <ul
        class="absolute max-h-40 scrollbar-hide overflow-y-auto scrollbar-none text-darkSlate-50 pt-1 z-20 shadow-md shadow-darkSlate-900 list-none m-0 p-0 w-full z-20"
        onMouseOut={() => {
          setFocusIn(false);
        }}
        onMouseOver={() => {
          setFocusIn(true);
        }}
        classList={{
          block: menuOpened(),
          hidden: !menuOpened(),
          "-left-10": props.btnDropdown,
          "min-w-20": props.btnDropdown,
        }}
      >
        <For each={props.options}>
          {(option) => (
            <li
              class="first:rounded-t last:rounded-b bg-darkSlate-700 hover:bg-[#343946] py-2 px-4 block whitespace-no-wrap text-darkSlate-50 no-underline"
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
};

const DropDownButton = (props: DropDownButtonProps) => {
  const handleChange = (option: Option) => {
    props.onChange?.(option.label);
  };

  return (
    <div class="flex">
      <Button class="rounded-r-0 pr-0 flex gap-1">
        <span>{props.children}</span>
      </Button>
      <Dropdown
        btnDropdown
        class="rounded-l-0 h-11 pl-0"
        options={props.options}
        rounded
        bgColorClass="bg-primary-500"
        value={props.value}
        onChange={(option) => handleChange(option)}
      />
    </div>
  );
};

Dropdown.button = DropDownButton;

export { Dropdown };
