import { children, JSXElement, Match, mergeProps, Switch } from "solid-js";
import { VirtualContainer } from "@minht11/solid-virtual-container";

export interface ItemSize {
  width: number;
  height: number;
}
export interface Props {
  children: JSXElement[] | JSXElement;
  type?: "list" | "grid";
  cols?: number;
  itemSize?: ItemSize;
}

const ListItem = (props: any) => (
  <div
    style={props.style}
    class="w-full h-full"
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

  const c = children(() => props.children);

  let scrollTargetElement!: HTMLDivElement;

  return (
    <div ref={scrollTargetElement}>
      <Switch>
        <Match when={mergedProps.type === "grid"}>
          <VirtualContainer
            items={c() as JSXElement[]}
            scrollTarget={scrollTargetElement}
            itemSize={mergedProps.itemSize}
            crossAxisCount={() => {
              return props.cols || 3;
            }}
          >
            {ListItem}
          </VirtualContainer>
        </Match>
        <Match when={mergedProps.type === "list"}>
          <VirtualContainer
            items={c() as JSXElement[]}
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
