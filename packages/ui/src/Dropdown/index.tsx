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
import { cva, type VariantProps } from "class-variance-authority";

type Option = {
  label: string | JSX.Element | Element;
  key: string | number;
};

const dropdownButton = cva(
  "flex justify-between transition-all transition-200 ease-in-out font-semibold py-2 items-center min-h-10 box-border",
  {
    variants: {
      error: {
        true: "border-2 border-solid border-red-500",
        false: "border-0",
      },
      disabled: {
        true: "text-darkSlate-50 cursor-not-allowed",
        false: "",
      },
      menuOpened: {
        true: "text-lightSlate-50 outline outline-offset-2 outline-darkSlate-500 hover:outline-darkSlate-500",
        false:
          "text-darkSlate-50 hover:text-lightSlate-50 outline-none hover:outline-darkSlate-600",
      },
      rounded: {
        true: "rounded-full",
        false: "rounded-md",
      },
      btnDropdown: {
        true: "group-hover:bg-primary-300 border-l-1 border-solid border-primary-300 bg-primary-500 duration-100 hover:bg-primary-300",
        false: "group px-4",
      },
    },
    compoundVariants: [
      {
        disabled: false,
        btnDropdown: false,
        class: "bg-darkSlate-700",
      },
      {
        disabled: true,
        btnDropdown: false,
        class: "bg-darkSlate-800",
      },
    ],
    defaultVariants: {
      error: false,
      disabled: false,
      menuOpened: false,
      rounded: false,
      btnDropdown: false,
    },
  }
);

type DropdownButtonProps = VariantProps<typeof dropdownButton>;

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
} & DropdownButtonProps;

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
  const incomingSelectedValue = () =>
    props.value
      ? props.options?.find((option) => option.key === props.value)
      : props.options[0];

  const [selectedValue, setSelectedValue] = createSignal<Option | undefined>(
    // eslint-disable-next-line solid/reactivity
    incomingSelectedValue()
  );

  const [menuOpened, setMenuOpened] = createSignal(false);
  const [focusIn, setFocusIn] = createSignal(false);
  const [buttonRef, setButtonRef] = createSignal<
    HTMLButtonElement | undefined
  >();
  const [menuRef, setMenuRef] = createSignal<HTMLUListElement | undefined>();

  createEffect(() => {
    setSelectedValue(incomingSelectedValue());
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
    middleware: [offset(10), flip(), shift(), hide(), size()],
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
        (option) => option.key === selectedValue()?.key
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
      <div class={`block relative ${props.containerClass || ""}`} id={props.id}>
        <button
          class={dropdownButton({
            error: !!props.error,
            disabled: props.disabled,
            menuOpened: menuOpened(),
            rounded: props.rounded,
            btnDropdown: props.btnDropdown,
            class: `${props.class} ${props.bgColorClass} ${props.textColorClass}`,
          })}
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
        >
          <Show when={!props.btnDropdown}>
            <Show when={props.icon}>
              <span class="mr-2">{props.icon}</span>
            </Show>
            <div class="w-[calc(100%-2rem)] flex justify-between">
              {selectedValue()?.label ?? props.placeholder}
            </div>
          </Show>
          <div class="i-ri:arrow-drop-down-line w-8 h-8 ease-in-out duration-100" />
        </button>
        <Show when={menuOpened()}>
          <Portal>
            <ul
              ref={setMenuRef}
              class="absolute h-max max-h-60 bottom-0 overflow-y-auto overflow-x-hidden text-darkSlate-50 shadow-md shadow-darkSlate-900 list-none m-0 p-0 z-100 min-w-32 max-w-200"
              onMouseOut={() => {
                setFocusIn(false);
              }}
              onMouseOver={() => {
                setFocusIn(true);
              }}
              style={{
                width: buttonRef()?.offsetWidth + "px" || "auto",
                "max-width": buttonRef()?.offsetWidth + "px" || "auto",
                top: `${position.y ?? 0}px`,
                left: `${position.x ?? 0}px`,
              }}
            >
              <For each={props.options}>
                {(option) => (
                  <li
                    class="first:rounded-t last:rounded-b hover:bg-darkSlate-800 py-2 px-4 block break-all text-darkSlate-50 no-underline w-full box-border"
                    classList={{
                      "bg-darkSlate-700": selectedValue()?.key !== option.key,
                      "bg-darkSlate-800": selectedValue()?.key === option.key,
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
