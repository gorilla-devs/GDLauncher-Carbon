import { autoUpdate, flip, hide, offset, shift, size } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import {
  Show,
  JSX,
  splitProps,
  createSignal,
  For,
  createEffect,
  onCleanup,
} from "solid-js";
import { Portal } from "solid-js/web";

type AutoCompleteOption = {
  value: string;
  label: JSX.Element | string;
};

interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {
  error?: string | boolean;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  inputColor?: string;
  icon?: Element | any;
  autoCompleteOptions?: AutoCompleteOption[];
  onSearch?: (_value: string) => void;
}

function Input(props: Props) {
  const [local, otherProps] = splitProps(props, [
    "class",
    "icon",
    "inputClass",
    "disabled",
    "error",
    "inputColor",
    "onBlur",
    "onFocus",
    "onInput",
    "onMouseDown",
    "onSearch",
  ]);

  const [menuOpened, setMenuOpened] = createSignal(false);
  const [focusIn, setFocusIn] = createSignal(false);
  const [inputRef, setInputRef] = createSignal<HTMLInputElement | undefined>();
  const [menuRef, setMenuRef] = createSignal<HTMLUListElement | undefined>();

  const toggleMenu = () => {
    if (props.disabled) return;
    setMenuOpened(!menuOpened());
  };

  const position = useFloating(inputRef, menuRef, {
    placement: "bottom-start",
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
    if (menuOpened() && menuRef() && props.autoCompleteOptions) {
      const selectedOptionIndex = props.autoCompleteOptions.findIndex(
        (option) => option.value === inputRef()?.value
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
          ref={setInputRef}
          class={`${inputBaseClasses} ${disabledClasses} ${errorClasses} ${inputColorClasses}`}
          disabled={local.disabled}
          onInput={(e) => {
            if (props.autoCompleteOptions && !menuOpened()) {
              toggleMenu();
            }

            if (props.onSearch) {
              props.onSearch((e.target as HTMLInputElement).value);
            }

            if (props.onInput) {
              props.onInput(_);
            }
          }}
          onFocus={() => {
            if (props.autoCompleteOptions && !menuOpened()) {
              toggleMenu();
            }

            if (props.onFocus) {
              props.onFocus();
            }
          }}
          onMouseDown={() => {
            if (props.autoCompleteOptions && !menuOpened()) {
              toggleMenu();
            }

            if (props.onMouseDown) {
              props.onMouseDown();
            }
          }}
          onBlur={() => {
            if (!focusIn()) {
              setMenuOpened(false);
            }

            if (props.onBlur) {
              props.onBlur();
            }
          }}
          {...otherProps}
        />
        <Show when={local.icon}>
          <span class="text-darkSlate-300">{local.icon}</span>
        </Show>
      </div>

      <Show when={local.error}>
        <div class="text-red-500 text-left mt-2 font-light">{local.error}</div>
      </Show>

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
            style={{
              "min-width": inputRef()?.offsetWidth + "px" || "auto",
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
          >
            <For each={props.autoCompleteOptions}>
              {(option) => (
                <li
                  class="first:rounded-t last:rounded-b bg-darkSlate-700 hover:bg-darkSlate-800 py-2 px-4 block whitespace-no-wrap text-darkSlate-50 no-underline w-full box-border"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopImmediatePropagation();
                    e.stopPropagation();
                    inputRef()?.focus();
                    if (inputRef() && inputRef()?.value !== option.value) {
                      inputRef()!.value = option.value;
                    }
                    toggleMenu();

                    if (props.onSearch) {
                      props.onSearch(option.value);
                    }
                  }}
                >
                  {option.label}
                </li>
              )}
            </For>
          </ul>
        </Portal>
      </Show>
    </>
  );
}

export { Input };
