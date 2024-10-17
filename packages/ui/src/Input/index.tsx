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
import { cva, type VariantProps } from "class-variance-authority";

type AutoCompleteOption = {
  value: string;
  label: JSX.Element | string;
};

const input = cva(
  "h-full w-full box-border py-2 rounded-md placeholder:text-darkSlate-400 outline-none",
  {
    variants: {
      errorMessage: {
        true: "border-2 border-solid border-red-500",
        false:
          "border-0 border-transparent hover:border-darkSlate-500 active:border-darkSlate-500",
      },
      disabled: {
        true: "text-darkSlate-300",
        false: "text-lightSlate-50",
      },
      hasIcon: {
        true: "",
        false: "px-4",
      },
    },
    compoundVariants: [
      {
        hasIcon: true,
        class: "bg-darkSlate-700",
      },
    ],
    defaultVariants: {
      errorMessage: false,
      disabled: false,
      hasIcon: false,
    },
  }
);

const container = cva(
  "outline-none has-[:focus-visible]:outline-darkSlate-500 hover:outline-darkSlate-600 hover:has-[:focus-visible]:outline-darkSlate-500 h-10 gap-2 box-border transition-all duration-200 rounded-md ease-in-out",
  {
    variants: {
      hasIcon: {
        true: "flex items-center px-4 bg-darkSlate-700",
        false: "",
      },
    },
    defaultVariants: {
      hasIcon: false,
    },
  }
);

interface Props
  extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "disabled">,
    Omit<VariantProps<typeof input>, "errorMessage"> {
  class?: string;
  inputClass?: string;
  inputColor?: string;
  icon?: Element | any;
  autoCompleteOptions?: AutoCompleteOption[];
  onSearch?: (_value: string) => void;
  containerClass?: string;
  disabled?: boolean;
  errorMessage?: string;
}

function Input(props: Props) {
  const [local, otherProps] = splitProps(props, [
    "class",
    "icon",
    "inputClass",
    "disabled",
    "inputColor",
    "onBlur",
    "onFocus",
    "onInput",
    "onMouseDown",
    "onSearch",
  ]);

  const [menuOpened, setMenuOpened] = createSignal(false);
  const [focusInUl, setFocusInUl] = createSignal(false);
  const [focusInInput, setFocusInInput] = createSignal(false);
  const [inputRef, setInputRef] = createSignal<HTMLInputElement | undefined>();
  const [menuRef, setMenuRef] = createSignal<HTMLUListElement | undefined>();
  const [inputContainerRef, setInputContainerRef] = createSignal<
    HTMLDivElement | undefined
  >();

  const position = useFloating(inputContainerRef, menuRef, {
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

  const focusIn = () => focusInUl() || focusInInput();

  return (
    <>
      <div
        class={container({
          hasIcon: !!local.icon,
          class: `${local.class || ""} ${local.inputColor || ""}`,
        })}
        ref={setInputContainerRef}
      >
        <input
          ref={setInputRef}
          class={input({
            errorMessage: !!props.errorMessage,
            disabled: !!local.disabled,
            hasIcon: !!local.icon,
            class: `${local.inputClass || ""} ${
              local.inputColor || "bg-darkSlate-600"
            }`,
          })}
          disabled={local.disabled}
          onInput={(e) => {
            if (props.disabled) return;

            if (props.autoCompleteOptions && !menuOpened()) {
              setMenuOpened(true);
            }

            if (props.onSearch) {
              props.onSearch((e.target as HTMLInputElement).value);
            }

            if (props.onInput && typeof props.onInput === "function") {
              props.onInput(e);
            }
          }}
          onFocus={(e) => {
            if (props.disabled) return;
            if (props.autoCompleteOptions && !menuOpened()) {
              setMenuOpened(true);
            }

            if (props.onSearch) {
              props.onSearch((e.target as HTMLInputElement).value);
            }

            if (props.onFocus && typeof props.onFocus === "function") {
              props.onFocus(e);
            }
          }}
          onMouseDown={(e) => {
            if (props.disabled) return;
            if (props.autoCompleteOptions && !menuOpened()) {
              setMenuOpened(true);
            }

            if (props.onSearch) {
              props.onSearch((e.target as HTMLInputElement).value);
            }

            if (props.onMouseDown && typeof props.onMouseDown === "function") {
              props.onMouseDown(e);
            }
          }}
          onBlur={(e) => {
            if (!focusIn()) {
              setMenuOpened(false);
              setFocusInInput(false);
            }

            if (props.onBlur && typeof props.onBlur === "function") {
              props.onBlur(e);
            }
          }}
          onMouseOver={() => {
            setFocusInInput(true);
          }}
          onMouseOut={() => {
            setFocusInInput(false);
          }}
          {...otherProps}
        />
        <Show when={local.icon}>
          <span class="text-darkSlate-300">{local.icon}</span>
        </Show>
      </div>

      <Show when={props.errorMessage}>
        <div class="text-red-500 text-left mt-2 font-light">
          {props.errorMessage}
        </div>
      </Show>

      <Show when={menuOpened()}>
        <Portal>
          <ul
            ref={setMenuRef}
            class="absolute h-max max-h-60 bottom-0 overflow-y-auto overflow-x-hidden text-darkSlate-50 shadow-md shadow-darkSlate-900 list-none m-0 p-0 z-100 min-w-32 max-w-200"
            onMouseOut={() => {
              setFocusInUl(false);
            }}
            onMouseOver={() => {
              setFocusInUl(true);
            }}
            style={{
              width: inputContainerRef()?.offsetWidth + "px" || "auto",
              "max-width": inputContainerRef()?.offsetWidth + "px" || "auto",
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
          >
            <For each={props.autoCompleteOptions}>
              {(option) => (
                <li
                  class="first:rounded-t last:rounded-b bg-darkSlate-700 hover:bg-darkSlate-800 py-2 px-4 block break-all text-darkSlate-50 no-underline w-full box-border"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopImmediatePropagation();
                    e.stopPropagation();
                    inputRef()?.focus();
                    if (inputRef() && inputRef()?.value !== option.value) {
                      inputRef()!.value = option.value;
                    }
                    setMenuOpened(false);
                    setFocusInInput(false);
                    setFocusInUl(false);

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
