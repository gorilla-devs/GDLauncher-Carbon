import { JSX } from "solid-js";

interface Props extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  value?: string | number;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  icon?: Element;
  onInput?: (_e: InputEvent) => void;
}

const TextArea = (props: Props) => {
  return (
    <textarea
      {...props}
      class={`bg-darkSlate-800 border-0 w-full min-h-20 rounded-md outline-none focus-visible:outline-darkSlate-500 hover:border-darkSlate-500 active:border-darkSlate-500 text-white py-2 px-3 box-border ${
        props.class || ""
      }`}
    >
      {props.children}
    </textarea>
  );
};

export { TextArea };
