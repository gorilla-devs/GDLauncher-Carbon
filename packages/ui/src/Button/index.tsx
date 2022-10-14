import { children, createSignal, createEffect } from "solid-js";

type Props = {
  children: HTMLElement | string;
};

function Button(props: Props) {
  const [q, setC] = createSignal(false);

  createEffect(() => {
    if (q()) {
      console.log(q());
    }
  });

  const c = children(() => props.children);
  return <div class={`font-main bg-slate-400 text-green-400 py-5`}>{c()}</div>;
}

export { Button };
