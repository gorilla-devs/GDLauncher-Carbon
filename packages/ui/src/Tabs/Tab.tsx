import { Match, Switch, createSignal, JSXElement, onMount } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: JSXElement | string | number;
  onClick?: (_: number) => void;
}

const Tab = (props: Props) => {
  const [index, setIndex] = createSignal(-1);
  const [ref, setRef] = createSignal<HTMLDivElement>();

  const tabsContext = useTabsContext();

  onMount(() => {
    // eslint-disable-next-line solid/reactivity
    queueMicrotask(() => {
      if (tabsContext) {
        setIndex(tabsContext.registerTab(ref()!));
      }
    });
  });

  return (
    <div
      class="cursor-pointer"
      classList={{
        "w-full": tabsContext?.variant === "block",
      }}
      ref={setRef}
      onClick={() => {
        props?.onClick?.(index());
        tabsContext?.setSelectedIndex(index());
      }}
    >
      <Switch>
        <Match when={tabsContext?.variant === "underline"}>
          <div
            class="cursor-pointer bg-shade-8 font-500 capitalize"
            classList={{
              "py-5": tabsContext?.orientation === "horizontal",
              "px-4": tabsContext?.orientation === "vertical",
              "text-white": tabsContext?.isSelectedIndex(index()),
              "text-shade-0": !tabsContext?.isSelectedIndex(index()),
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant === "block"}>
          <div
            class="flex gap-1 justify-center items-center bg-shade-8 flex-1 h-full cursor-pointer rounded-xl font-500 capitalize box-border"
            classList={{
              "py-5 px-2": tabsContext?.orientation === "horizontal",
              "px-4 py-2": tabsContext?.orientation === "vertical",
              "text-white": tabsContext?.isSelectedIndex(index()),
              "text-shade-0": !tabsContext?.isSelectedIndex(index()),
            }}
          >
            {props.children}
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export { Tab };
