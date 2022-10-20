import { children, createSignal, createEffect } from "solid-js";

type Props = {
  children: HTMLElement | string;
};

function Select(props: Props) {
  const [q, setC] = createSignal(false);

  createEffect(() => {
    if (q()) {
      console.log(q());
    }
  });

  const c = children(() => props.children);
  return <div class={`font-main bg-slate-500 py-5`}>{c()}</div>;
}

export { Select };
