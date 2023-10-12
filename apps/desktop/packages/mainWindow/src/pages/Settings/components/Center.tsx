import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
  class?: string;
};

function RightHandSide(props: Props) {
  const c = children(() => props.children);

  return (
    <div class={"flex gap-4 justify-center items-center w-full " + props.class}>
      {c()}
    </div>
  );
}

export default RightHandSide;
