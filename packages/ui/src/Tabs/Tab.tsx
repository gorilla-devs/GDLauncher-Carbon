import { Match, Switch, createSignal } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | string | number;
  // eslint-disable-next-line no-unused-vars
  onClick?: (index: number) => void;
}

const Tab = (props: Props) => {
  const [index, setIndex] = createSignal(-1);

  const tabsContext = useTabsContext();

  return (
    <div
      class="cursor-pointer"
      ref={(ref: HTMLDivElement) => {
        if (tabsContext) {
          setIndex(tabsContext.registerTab(ref));
        }
      }}
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
              "py-4": tabsContext?.orientation === "horizontal",
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
            class="flex pr-4 gap-1 justify-center items-center flex-1 h-full  cursor-pointer rounded-xl font-500 capitalize box-border"
            classList={{
              "py-4": tabsContext?.orientation === "horizontal",
              "px-4": tabsContext?.orientation === "vertical",
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
