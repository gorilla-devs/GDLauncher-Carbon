import { children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element | Element;
  class?: string;
}

// Page starts with opacity 0 and then fades in, handled in app.tsx
const Page = (props: Props) => {
  const c = children(() => props.children);
  return (
    <div class="w-full h-full max-h-full flex justify-center overflow-auto box-border p-5 text-white bg-black-semiblack">
      <div class={`rounded-2xl h-full w-full box-border ${props.class ?? ""}`}>
        {c()}
      </div>
    </div>
  );
};

export default Page;
