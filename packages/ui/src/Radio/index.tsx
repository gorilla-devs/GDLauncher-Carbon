import { JSX, Show, createContext, useContext } from "solid-js";

interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {}

type OnChangeValue = string | number | string[] | undefined;

type OnChange = (_value: OnChangeValue) => void;

type RadioGroupContext = {
  onChange: OnChange;
};

type GroupProps = {
  onChange?: OnChange;
  children?: JSX.Element[];
};

const RadioContext = createContext<RadioGroupContext>();

const useRadioContext = () => {
  const context = useContext(RadioContext);

  return context as RadioGroupContext;
};

const Radio = (props: Props) => {
  const radioContext = useRadioContext();
  return (
    <label class="relative inline-flex items-center justify-between">
      <span class="cursor-pointer">
        <input
          type="radio"
          class="peer absolute opacity-0 cursor-pointer"
          {...props}
          onChange={() => radioContext?.onChange(props.value)}
        />
        <div class="relative box-border bg-shade-5 border-1 border-solid border-transparent hover:border-accent w-5 h-5 rounded-[50%] before:content-[] before:w-4 before:h-4 before:bg-accent before:opacity-0 peer-checked:before:opacity-100 before:rounded-full before:absolute before:top-1/2 before:left-1/2 before:-translate-x-1/2 before:-translate-y-1/2 transition ease-in-out" />
      </span>
      <Show when={props.children}>
        <span class="px-3">{props.children}</span>
      </Show>
    </label>
  );
};

const Group = (props: GroupProps) => {
  const context = {
    onChange: (value: OnChangeValue) => props?.onChange?.(value),
  };

  return (
    <RadioContext.Provider value={context}>
      {props.children}
    </RadioContext.Provider>
  );
};

Radio.group = Group;

export { Radio };
