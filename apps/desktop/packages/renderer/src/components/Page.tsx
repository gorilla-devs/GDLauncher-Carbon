import { children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element;
}

// Page starts with opacity 0 and then fades in, handled in app.tsx
const Page = (props: Props) => {
  const c = children(() => props.children);
  return <div class="absolute w-full h-full opacity-0">{c()}</div>;
};

export default Page;
