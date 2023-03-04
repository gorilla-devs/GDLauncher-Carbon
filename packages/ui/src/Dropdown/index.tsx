import { createSignal, For, Show, JSX, Switch, Match } from "solid-js";
import { Button } from "../Button";

export type Label = {
  name: string;
  icon: string;
};

export type Option = {
  label: string | Label;
  key: string;
};

export type OptionDropDown = {
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
  id?: string;
  bg?: string;
  btnDropdown?: boolean;
  icon?: JSX.Element;
};
export interface DropDownButtonProps extends Props {
  children: JSX.Element;
}

const Dropdown = (props: Props) => {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label ||
    props.options[0]?.label;

  const [selectedValue, setSelectedValue] = createSignal(defaultValue());
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
    <div class="inline-block relative" id={props.id}>
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
        class={`group flex justify-between font-semibold py-2 px-4 inline-flex items-center min-h-10 box-border ${props.class} ${props.bg}`}
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
          "text-shade-0 hover:text-white": !props.disabled && !props.error,
          "text-shade-5": props.error,
          "rounded-full": props.rounded,
          rounded: !props.rounded,
          "bg-shade-7": !props.bg,
        }}
      >
        <Show when={!props.btnDropdown}>
          <Show when={props.icon}>
            <span class="mr-2">{props.icon}</span>
          </Show>
          <Show when={(selectedValue() as Label).icon}>
            <img
              src={(selectedValue() as Label).icon}
              class="w-5 h-5 rounded-md mr-2"
            />
          </Show>
          <span
            class="w-full"
            classList={{
              "text-white": props.error,
              "text-shade-0 hover:text-white group-hover:text-white":
                !props.disabled && !props.error,
              "text-shade-5": props.disabled,
            }}
          >
            <Switch>
              <Match when={typeof selectedValue() === "string"}>
                {selectedValue() as string}
              </Match>
              <Match when={typeof selectedValue() === "object"}>
                {(selectedValue() as Label).name}
              </Match>
            </Switch>
          </span>
        </Show>
        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-shade-0 group-hover:text-white":
              !props.disabled && !props.error && !props.btnDropdown,
            "text-white": props.error || props.btnDropdown,
            "text-shade-5": props.disabled,
          }}
        />
      </button>

      <ul
        class={`absolute text-shade-0 pt-1 z-20 shadow-md shadow-shade-9 list-none m-0 p-0 w-full z-20`}
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
              class="first:rounded-t last:rounded-b bg-shade-7 hover:bg-[#343946] py-2 px-4 block whitespace-no-wrap text-shade-0 no-underline"
              onClick={() => {
                setSelectedValue(option.label);
                props.onChange?.(option);
                toggleMenu();
              }}
            >
              <Switch>
                <Match when={typeof option.label === "string"}>
                  {option.label as string}
                </Match>
                <Match when={typeof option.label === "object"}>
                  <img
                    src={(option.label as Label).icon}
                    class="w-5 h-5 rounded-md mr-2"
                  />
                  {(option.label as Label).name}
                </Match>
              </Switch>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
};

const DropDownButton = (props: DropDownButtonProps) => {
  const [selectedValue, setSelectedValue] = createSignal<string>("");

  const handleChange = (option: OptionDropDown) => {
    setSelectedValue(option.label);
  };

  return (
    <div class="flex">
      <Button class="rounded-r-0 pr-0 flex gap-1">
        <span>{props.children}</span>
        <span class="text-white font-light text-xs">{selectedValue()}</span>
      </Button>
      <Dropdown
        btnDropdown
        class="rounded-l-0 h-11 pl-0"
        options={props.options}
        rounded
        bg="bg-primary"
        value={props.value}
        onChange={(option) => handleChange(option as OptionDropDown)}
      />
    </div>
  );
};

Dropdown.button = DropDownButton;

export { Dropdown };
