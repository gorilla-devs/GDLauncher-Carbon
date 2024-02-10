import {
  createEffect,
  createSignal,
  For,
  JSX,
  onCleanup,
  Show,
} from "solid-js";
import { Button } from "../Button";
import { Portal } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import {
  autoUpdate,
  flip,
  hide,
  offset,
  Placement,
  shift,
  size,
} from "@floating-ui/dom";

type Option = {
  label: string | JSX.Element | Element;
  key: string | number;
};

type Props = {
  options: Option[];
  value?: string | number | null | undefined;
  error?: string | boolean;
  disabled?: boolean;
  rounded?: boolean;
  label?: string;
  onChange?: (_option: Option) => void;
  class?: string;
  containerClass?: string;
  id?: string;
  bgColorClass?: string;
  textColorClass?: string;
  btnDropdown?: boolean;
  icon?: JSX.Element;
  placeholder?: string;
  placement?: "bottom" | "top";
  menuPlacement?: Placement;
};

interface DropDownButtonProps {
  children: JSX.Element;
  options: Option[];
  value: string | number;
  error?: boolean;
  disabled?: boolean;
  loading?: boolean;
  rounded?: boolean;
  label?: string;
  onChange?: (_value: Option) => void;
  onClick?: () => void;
  class?: string;
  id?: string;
  bgColorClass?: string;
  btnDropdown?: boolean;
  icon?: JSX.Element;
  menuPlacement?: Placement;
}

const Dropdown = (props: Props) => {
  const defaultValue = () =>
    props.options?.find((option) => option.key === props.value) ||
    props.options?.[0];

  const [selectedValue, setSelectedValue] = createSignal<Option>(
    // eslint-disable-next-line solid/reactivity
    defaultValue() || props.placeholder
  );
  const [menuOpened, setMenuOpened] = createSignal(false);
  const [focusIn, setFocusIn] = createSignal(false);
  const [buttonRef, setButtonRef] = createSignal<
    HTMLButtonElement | undefined
  >();
  const [menuRef, setMenuRef] = createSignal<HTMLUListElement | undefined>();

  createEffect(() => {
    setSelectedValue(defaultValue());
  });

  const toggleMenu = () => {
    if (props.disabled) return;
    setMenuOpened(true);
    setTimeout(() => {
      setMenuOpened(false);
    }, 100);
  };

  const position = useFloating(buttonRef, menuRef, {
    placement: props.menuPlacement || "bottom-start",
    middleware: [offset(5), flip(), shift(), hide(), size()],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
  });

  createEffect(() => {
    if (position.middlewareData.hide?.referenceHidden) setMenuOpened(false);
  });

  onCleanup(() => setMenuOpened(false));

  createEffect(() => {
    if (menuOpened() && menuRef() && selectedValue()) {
      const selectedOptionIndex = props.options.findIndex(
        (option) => option.key === selectedValue().key
      );

      if (selectedOptionIndex !== -1) {
        const selectedOption = (menuRef() as HTMLUListElement).children[
          selectedOptionIndex
        ] as HTMLElement;

        const menuElement = menuRef() as HTMLElement;
        const menuRect = menuElement.getBoundingClientRect();
        const optionRect = selectedOption.getBoundingClientRect();

        const isOptionInView =
          optionRect.top >= menuRect.top &&
          optionRect.bottom <= menuRect.bottom;

        if (!isOptionInView) {
          const scrollMiddle =
            optionRect.top -
            menuRect.top +
            menuElement.scrollTop -
            (menuRect.height / 2 - optionRect.height / 2);
          menuElement.scrollTop = scrollMiddle;
        }
      }
    }
  });

  return (
    <>
      <div
        class={`inline-block relative ${props.containerClass || ""}`}
        id={props.id}
      >
        <button
          class={`flex justify-between font-semibold py-2 inline-flex items-center min-h-10 box-border ${props.class} ${props.bgColorClass} ${props.textColorClass}`}
          onClick={() => {
            if (props.disabled) return;
            setMenuOpened(!menuOpened());
          }}
          onBlur={() => {
            if (!focusIn()) {
              setMenuOpened(false);
            }
          }}
          ref={setButtonRef}
          classList={{
            "border-0": !props.error,
            "border-2 border-solid border-red-500": !!props.error,
            "text-darkSlate-50 hover:text-white":
              !props.disabled && !props.error && !props.textColorClass,
            "text-darkSlate-500": !!props.error && !props.textColorClass,
            "rounded-full": props.rounded,
            "bg-darkSlate-700": !props.bgColorClass && !props.disabled,
            "rounded-md": !props.btnDropdown && !props.rounded,
            "group-hover:bg-primary-300 border-l-1 border-solid border-primary-300":
              props.btnDropdown && !props.disabled,
            "group px-4": !props.btnDropdown,
            "bg-primary-500 duration-100": props.btnDropdown && !props.disabled,
            "hover:bg-primary-300": props.btnDropdown && !props.disabled,
            "cursor-pointer": !props.disabled,
            "cursor-not-allowed": props.disabled && !!props.bgColorClass,
            "cursor-not-allowed bg-darkSlate-800":
              props.disabled && !props.bgColorClass,
          }}
        >
          <Show when={!props.btnDropdown}>
            <Show when={props.icon}>
              <span class="mr-2">{props.icon}</span>
            </Show>
            <span
              class="w-full flex justify-between"
              classList={{
                "text-white": !!props.error,
                "text-darkSlate-50 hover:text-white group-hover:text-white":
                  !props.disabled && !props.error && !props.textColorClass,
                "text-darkSlate-500":
                  props.disabled && !props.textColorClass && !props.btnDropdown,
              }}
            >
              {selectedValue()?.label ?? props.placeholder}
            </span>
          </Show>
          <span
            class="i-ri:arrow-drop-down-line text-3xl ease-in-out duration-100"
            classList={{
              "text-darkSlate-50 group-hover:text-white":
                !props.disabled &&
                !props.error &&
                !props.btnDropdown &&
                !props.textColorClass,
              "text-white":
                !!props.error ||
                (props.btnDropdown && !props.textColorClass && !props.disabled),
              "text-darkSlate-500": props.disabled,
            }}
          />
        </button>
        <Show when={menuOpened()}>
          <Portal>
            <ul
              ref={setMenuRef}
              class="absolute h-max max-h-60 bottom-0 overflow-y-auto overflow-x-hidden text-darkSlate-50 shadow-md shadow-darkSlate-900 list-none m-0 p-0 z-100 min-w-32"
              onMouseOut={() => {
                setFocusIn(false);
              }}
              onMouseOver={() => {
                setFocusIn(true);
              }}
              classList={{
                "min-w-20": props.btnDropdown,
              }}
              style={{
                "min-width": buttonRef()?.offsetWidth + "px" || "auto",
                top: `${position.y ?? 0}px`,
                left: `${position.x ?? 0}px`,
              }}
            >
              <For each={props.options}>
                {(option) => (
                  <li
                    class="first:rounded-t last:rounded-b hover:bg-darkSlate-800 py-2 px-4 block whitespace-no-wrap text-darkSlate-50 no-underline w-full box-border max-w-100"
                    classList={{
                      "bg-darkSlate-700": selectedValue().key !== option.key,
                      "bg-darkSlate-800": selectedValue().key === option.key,
                    }}
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopImmediatePropagation();
                      e.stopPropagation();
                      setSelectedValue(option);
                      props.onChange?.(option);
                      toggleMenu();
                    }}
                  >
                    {option.label}
                  </li>
                )}
              </For>
            </ul>
          </Portal>
        </Show>
      </div>
      <Show when={props.error}>
        <div class="text-red-500 text-left mt-2 font-light">{props.error}</div>
      </Show>
    </>
  );
};

const DropDownButton = (props: DropDownButtonProps) => {
  const handleChange = (option: Option) => {
    props.onChange?.(option);
  };

  return (
    <div class="flex">
      <Button
        disabled={props.disabled}
        loading={props.loading}
        class="rounded-r-0 pr-4 pl-4 flex gap-1"
        onClick={() => {
          if (!props.disabled && !props.loading) props?.onClick?.();
        }}
      >
        <span>{props.children}</span>
      </Button>
      <Dropdown
        disabled={props.disabled}
        btnDropdown
        class="rounded-l-0 h-11 pl-0"
        options={props.options}
        rounded
        value={props.value}
        onChange={(option) => handleChange(option)}
        menuPlacement={props.menuPlacement}
      />
    </div>
  );
};

Dropdown.button = DropDownButton;

export { Dropdown };
