import { JSXElement, mergeProps } from "solid-js";

interface Props {
  children: JSXElement[];
  type?: "list" | "grid";
  cols?: number;
}

function ViewList(props: Props) {
  const mergedProps = mergeProps({ type: "grid", cols: 4 }, props);

  return (
    <div
      class={`${mergedProps.type === "grid" ? `grid` : "flex flex-col"} gap-4`}
      style={{
        "grid-template-columns": `repeat(${mergedProps.cols}, minmax(0, 1fr))`,
      }}
    >
      {props.children}
    </div>
  );
}

export { ViewList };
