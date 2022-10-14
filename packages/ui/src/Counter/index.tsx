import { createSignal } from "solid-js";
import type { Component } from "solid-js";
import { Button } from "../Button";

interface Props {
  initialValue?: number;
}

export const Counter: Component<Props> = (props) => {
  const [count, setCount] = createSignal(props.initialValue ?? 0);

  return (
    <div>
      <Button>Prova</Button>
      <div class="text-sky-500">Prova</div>
      <button onClick={() => setCount((c) => c + 1)}>Click</button>{" "}
      <span>{count()}</span>
    </div>
  );
};
