import { JSXElement, Match, mergeProps, Switch } from "solid-js";
import { VirtualContainer } from "@minht11/solid-virtual-container";

interface ItemSize {
  width: number;
  height: number;
}
interface Props {
  children: JSXElement[];
  type?: "list" | "grid";
  cols?: number;
  itemSize?: ItemSize;
}

const ListItem = (props: any) => (
  <div
    style={props.style}
    class="w-full border-black border-1"
    tabIndex={props.tabIndex}
    role="listitem"
  >
    <div>{props.item}</div>
  </div>
);

function ViewList(props: Props) {
  const mergedProps = mergeProps(
    { type: "grid", cols: 4, itemSize: { height: 50 } },
    props
  );

  let scrollTargetElement!: HTMLDivElement;

  return (
    <div
      ref={scrollTargetElement}
      style={{
        "grid-template-columns": `repeat(${mergedProps.cols}, minmax(0, 1fr))`,
      }}
    >
      <Switch>
        <Match when={mergedProps.type === "grid"}>
          <VirtualContainer
            items={props.children}
            scrollTarget={scrollTargetElement}
            itemSize={mergedProps.itemSize}
            crossAxisCount={(measurements) => {
              console.log(
                "TEST",
                measurements,
                measurements.container.cross / measurements.itemSize.cross
              );
              return props.cols || 3;
            }}
          >
            {ListItem}
          </VirtualContainer>
        </Match>
        <Match when={mergedProps.type === "list"}>
          <VirtualContainer
            items={props.children}
            scrollTarget={scrollTargetElement}
            itemSize={mergedProps.itemSize}
          >
            {ListItem}
          </VirtualContainer>
        </Match>
      </Switch>
    </div>
  );
}

export { ViewList };
