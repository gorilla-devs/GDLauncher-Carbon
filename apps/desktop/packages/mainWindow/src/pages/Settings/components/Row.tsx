import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
  class?: string;
};

function Row(props: Props) {
  const c = children(() => props.children);

  return (
    <div
      class={"flex justify-between items-center min-h-26 " + props.class || ""}
    >
      {c()}
    </div>
  );
}

export default Row;
