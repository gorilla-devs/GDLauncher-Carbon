import { JSX } from "solid-js";
export interface Props extends JSX.InputHTMLAttributes<HTMLInputElement> {}

function Switch(props: Props) {
  return (
    <label class="relative inline-block w-10 h-5 m-2">
      <input {...props} class="opacity-0 w-0 h-0 peer" type="checkbox" />
      <span
        class="absolute cursor-pointer top-0 left-0 right-0 bottom-0 transition-all duration-100 ease-in-out rounded-full before:absolute before:content-[] before:w-4	before:h-4 before:left-0.5 before:bottom-0.5 before:rounded-full peer-checked:before:translate-x-5 before:transition-all before:ease-in-out before:duration-100"
        classList={{
          "before:bg-darkSlate-800": true,
          "peer-checked:before:bg-white": true,
          "peer-checked:bg-primary": true,
          "bg-darkSlate-500": true,
          "peer-disabled:before:bg-darkSlate-50": true,
          "peer-disabled:bg-shade-9": true,
        }}
      />
    </label>
  );
}

export { Switch };
