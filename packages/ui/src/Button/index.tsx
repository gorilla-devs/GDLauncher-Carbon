import { children } from "solid-js";

type Props = {
  children: HTMLElement | string;
  class?: string;
  onClick?: () => void;
};
function Button(props: Props) {
  const c = children(() => props.children);
  return (
    <div
      class={`font-main bg-accent-main text-white py-4 px-8 rounded-full cursor-pointer uppercase font-bold ${
        props.class || ""
      }`}
      onClick={props.onClick}
    >
      {c()}
    </div>
  );
}

export { Button };
