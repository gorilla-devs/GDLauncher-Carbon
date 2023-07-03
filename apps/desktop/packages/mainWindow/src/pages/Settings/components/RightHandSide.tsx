import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
  class?: string;
};

function RightHandSide(props: Props) {
  const c = children(() => props.children);

  return <div class={props.class}>{c()}</div>;
}

export default RightHandSide;
