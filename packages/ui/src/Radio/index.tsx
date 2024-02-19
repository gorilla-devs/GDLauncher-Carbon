import { JSX, Show, Switch, Match, For } from "solid-js";
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
  const id = `radio-${nextId++}`;

  // Determine base and conditional classes based on buttonStyle
  const baseClasses = "relative flex gap-3 items-center";
  const indicatorBaseClasses =
    "w-4 h-4 rounded-full border border-gray-300 bg-white";
  const indicatorCheckedClasses = "bg-blue-500 border-transparent";

  return (
    <>
      <input
        type="radio"
        class="hidden"
        {...props}
        checked={props.checked}
        id={id}
      />
      <label
        for={id}
        class={`${baseClasses}`}
        onClick={() => {
          props?.onChange(props.value);
        }}
      >
        <Show when={props?.buttonStyle === "button"}>
          <Button type={props.checked ? "primary" : "secondary"}>
            {props.children}
          </Button>
        </Show>
        <Show when={props?.buttonStyle !== "button"}>
          <div
            class={`flex justify-center items-center ${indicatorBaseClasses} ${
              props.checked ? indicatorCheckedClasses : ""
            }`}
          >
            <Show when={props.checked}>
              <div class="w-2 h-2 bg-white rounded-full"></div>
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
        <div class="flex flex-col">
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
    </Switch>
  );
};

Radio.group = Group;

export { Radio };
