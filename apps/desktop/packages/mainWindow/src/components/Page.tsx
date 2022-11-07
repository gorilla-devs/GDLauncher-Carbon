import { children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element;
  class: string;
}

// Page starts with opacity 0 and then fades in, handled in app.tsx
const Page = (props: Props) => {
  const c = children(() => props.children);
  return (
    <div class="w-full h-full opacity-0 box-border p-5 bg-[#272B35]">
      <div class={`rounded-2xl h-full w-full box-border ${props.class ?? ""}`}>
        {c()}
      </div>
    </div>
  );
};

export default Page;
