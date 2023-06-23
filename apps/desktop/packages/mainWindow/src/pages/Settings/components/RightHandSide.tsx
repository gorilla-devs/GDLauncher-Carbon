import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
};

function RightHandSide(props: Props) {
  const c = children(() => props.children);

  return <h4>{c()}</h4>;
}

export default RightHandSide;
