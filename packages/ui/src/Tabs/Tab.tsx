import { Match, Show, Switch, createSignal } from "solid-js";
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
      ref={(el: HTMLDivElement) => {
        if (tabsContext) {
          setIndex(tabsContext.registerTab(el));
        }
      }}
      onClick={() => {
        props?.onClick?.(index());
        tabsContext?.setSelectedIndex(index());
      }}
    >
      <Switch>
        <Match when={tabsContext?.type === "underline"}>
          <div
            class={`cursor-pointer relative bg-shade-8 font-500 capitalize ${
              tabsContext?.isSelectedIndex(index())
                ? "text-white"
                : "text-shade-0"
            }`}
            classList={{
              "py-4": tabsContext?.orientation === "horizontal",
              "px-4": tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
            <Show when={tabsContext?.isSelectedIndex(index())}>
              <div class="absolute left-0 right-0 bottom-0 h-1 bg-primary" />
            </Show>
          </div>
        </Match>
        <Match when={tabsContext?.type === "block"}>
          <div
            class={`flex pr-4 gap-1 justify-center items-center flex-1 h-full  cursor-pointer rounded-xl font-500 capitalize box-border ${
              tabsContext?.isSelectedIndex(index())
                ? "text-white bg-black-semiblack"
                : "text-shade-0"
            }`}
            classList={{
              "py-4": tabsContext?.orientation === "horizontal",
              "px-4": tabsContext?.orientation === "vertical",
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
