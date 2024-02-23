import {
  JSX,
  Show,
  Switch,
  Match,
  For,
  splitProps,
  createSignal,
} from "solid-js";
import { Button } from "../Button";

type Props = {
  value: string | number | string[] | undefined;
  checked: boolean;
  onChange?: OnChange;
  buttonStyle?: ButtonStyle;
  children?: JSX.Element;
};

type OnChange = (_value: string | number | string[] | undefined) => void;

type Option = {
  value: string | number | string[] | undefined;
  label: JSX.Element;
};

type ButtonStyle = "standard" | "button";

type GroupProps = {
  onChange?: OnChange;
  value?: string | number | string[] | undefined;
  buttonStyle?: ButtonStyle;
  options: Option[];
};

let nextId = 1;

const Radio = (props: Props) => {
  const [local, otherProps] = splitProps(props, ["buttonStyle", "onChange"]);
  const [isHovered, setIsHovered] = createSignal(false); // Track hover state

  const id = `radio-${nextId++}`;

  // Determine base and conditional classes based on buttonStyle
  const baseClasses = "relative flex gap-3 items-center";
  const indicatorBaseClasses = "w-5 h-5 min-w-5 min-h-5 rounded-full";
  const indicatorCheckedClasses = "border-transparent";

  return (
    <>
      <input
        type="radio"
        class="hidden"
        {...otherProps}
        checked={props.checked}
        id={id}
      />
      <label
        for={id}
        class={`${baseClasses}`}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        onClick={() => {
          local?.onChange?.(props.value);
        }}
      >
        <Show when={local?.buttonStyle === "button"}>
          <Button type={props.checked ? "primary" : "secondary"}>
            {props.children}
          </Button>
        </Show>
        <Show when={local?.buttonStyle !== "button"}>
          <div
            class={`flex justify-center items-center bg-darkSlate-500 box-border ${indicatorBaseClasses} ${
              props.checked ? indicatorCheckedClasses : ""
            } ${
              isHovered()
                ? "border-darkSlate-300 border-solid border-1 border"
                : ""
            }`}
          >
            <Show when={props.checked}>
              <div class="w-4 h-4 rounded-full bg-blue-500"></div>
            </Show>
          </div>
          <Show when={props.children}>
            <span class="ml-2">{props.children}</span>
          </Show>
        </Show>
      </label>
    </>
  );
};

const Group = (props: GroupProps) => {
  return (
    <Switch>
      <Match when={props.buttonStyle === "button"}>
        <div class="flex bg-darkSlate-900 w-max">
          <For each={props.options}>
            {(option) => (
              <Radio
                value={option.value}
                checked={props.value === option.value}
                onChange={props.onChange}
                buttonStyle={props.buttonStyle}
              >
                {option.label}
              </Radio>
            )}
          </For>
        </div>
      </Match>
      <Match when={props.buttonStyle !== "button"}>
        <For each={props.options}>
          {(option) => (
            <Radio
              value={option.value}
              checked={props.value === option.value}
              onChange={props.onChange}
              buttonStyle={props.buttonStyle}
            >
              {option.label}
            </Radio>
          )}
        </For>
      </Match>
    </Switch>
  );
};

Radio.group = Group;

export { Radio };
