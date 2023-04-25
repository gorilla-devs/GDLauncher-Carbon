import { JSX, Show } from "solid-js";

interface Props extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  value?: string | number;
  placeholder?: string;
  error?: string | boolean;
  disabled?: boolean;
  class?: string;
  inputClass?: string;
  icon?: Element;
}

const TextArea = (props: Props) => {
  return (
    <>
      <textarea
        {...props}
        class={`bg-darkSlate-800 border-0 w-full min-h-20 rounded-md outline-none focus-visible:outline-darkSlate-500 text-white py-2 px-3 box-border placeholder:text-darkSlate-500 ${
          props.class || ""
        }`}
        classList={{
          "border-0 border-transparent hover:border-darkSlate-500 active:border-darkSlate-500":
            !props.error,
          "border-2 border-solid border-red-500": !!props.error,
        }}
      >
        {props.children}
      </textarea>
      <Show when={props.error}>
        <div class="text-red-500 text-left mt-2 font-light">{props.error}</div>
      </Show>
    </>
  );
};

export { TextArea };
