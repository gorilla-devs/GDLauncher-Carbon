import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
  class?: string;
  forceContentBelow?: boolean;
};

function Row(props: Props) {
  const c = children(() => props.children);

  return (
    <div
      class={
        "flex justify-between items-center min-h-26 " +
          (props.forceContentBelow ? " flex-col items-stretch " : "") +
          props.class || ""
      }
    >
      {c()}
    </div>
  );
}

export default Row;
