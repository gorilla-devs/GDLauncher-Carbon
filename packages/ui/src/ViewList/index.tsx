import { For, mergeProps } from "solid-js";

interface Props {
  list: any[];
  type?: "list" | "grid";
  cols: number;
}

function ViewList(props: Props) {
  const mergedProps = mergeProps({ type: "grid" }, props);

  return (
    <div
      class={`${mergedProps.type === "grid" ? `grid` : "flex flex-col"} gap-4`}
      style={{
        "grid-template-columns": `repeat(${props.cols}, minmax(0, 1fr))`,
      }}
    >
      <For each={props.list}>
        {() => <div class="bg-green-500 rounded-xl h-20" />}
      </For>
    </div>
  );
}

export { ViewList };
