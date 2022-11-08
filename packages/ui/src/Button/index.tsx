import { children, createSignal, createEffect } from "solid-js";

type Props = {
  children: HTMLElement | string;
  onClick: () => void;
};
function Button(props: Props) {
  const [q, setC] = createSignal(false);

  createEffect(() => {
    if (q()) {
      console.log(q());
    }
  });

  const c = children(() => props.children);
  return (
    <div
      class="font-main bg-accent-main text-white py-4 px-8 rounded-full cursor-pointer uppercase font-bold"
      onClick={props.onClick}
    >
      {c()}
    </div>
  );
}

export { Button };
