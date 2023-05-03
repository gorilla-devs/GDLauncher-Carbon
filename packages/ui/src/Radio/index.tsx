import {
  JSX,
  Show,
  children,
  createContext,
  createEffect,
  useContext,
} from "solid-js";

interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {}

type OnChange = (_value: string | number | string[] | undefined) => void;

type RadioGroupContext = {
  onChange: OnChange;
};

type GroupProps = {
  onChange?: OnChange;
  children: JSX.Element[];
  value?: string | number | string[] | undefined;
};

const RadioContext = createContext<RadioGroupContext>();

const useRadioContext = () => {
  const context = useContext(RadioContext);

  return context as RadioGroupContext;
};

const Radio = (props: Props) => {
  const radioContext = useRadioContext();
  return (
    <label class="relative flex gap-3 items-center">
      <span class="cursor-pointer">
        <input
          type="radio"
          class="peer absolute opacity-0 cursor-pointer"
          {...props}
          onChange={() => radioContext?.onChange(props.value)}
        />
        <div class="relative box-border bg-darkSlate-500 peer-disabled:bg-darkSlate-900 peer-disabled:border-0 border-1 border-solid border-transparent hover:border-primary-300 w-5 h-5 rounded-[50%] before:content-[] before:w-4 before:h-4 before:bg-primary-300 before:opacity-0 peer-checked:before:opacity-100 before:rounded-full before:absolute before:top-1/2 before:left-1/2 before:-translate-x-1/2 before:-translate-y-1/2 transition ease-in-out" />
      </span>
      <Show when={props.children}>
        <span>{props.children}</span>
      </Show>
    </label>
  );
};

const Group = (props: GroupProps) => {
  const context = {
    onChange: (value: string | number | string[] | undefined) =>
      props?.onChange?.(value),
  };

  const c = children(() => props.children);

  createEffect(() => {
    console.log("AAAA", props.value);
    (c() as JSX.InputHTMLAttributes<HTMLInputElement>[])?.forEach((item) => {
      // @ts-ignore
      const input = item.querySelector(".cursor-pointer input");
      console.log("VAL", props.value, input.value, props.value === input.value);
      item.checked = props.value === input.value;
    });
  });

  return <RadioContext.Provider value={context}>{c()}</RadioContext.Provider>;
};

Radio.group = Group;

export { Radio };
