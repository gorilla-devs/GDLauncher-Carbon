import { JSX, children } from "solid-js";

type Props = {
  id?: string;
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
      {...(props.id ? { id: props.id } : {})}
    >
      {c()}
    </div>
  );
}

export default Row;
