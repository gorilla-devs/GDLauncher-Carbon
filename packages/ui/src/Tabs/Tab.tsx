import { Match, Show, Switch, createSignal } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | string | number;
}

const Tab = (props: Props) => {
  const [index, setIndex] = createSignal(-1);

  const tabsContext = useTabsContext();

  return (
    <div
      class="cursor-pointer h-10 w-10"
      ref={(el: HTMLDivElement) => {
        setIndex(tabsContext?.registerTab(el));
      }}
      onClick={() => {
        tabsContext?.setSelectedIndex(index());
      }}
    >
      <Switch>
        <Match when={tabsContext?.type === "underline"}>
          <div
            class={`py-4 cursor-pointer min-w-fit relative font-500 capitalize ${
              tabsContext.isSelectedIndex() ? "text-white" : "text-shade-0"
            }`}
            // onClick={() => handleClick(i())}
          >
            {props.children}
            <Show when={tabsContext.isSelectedIndex()}>
              <div class="absolute left-0 right-0 bottom-0 h-1 bg-primary" />
            </Show>
          </div>
        </Match>
        <Match when={tabsContext?.type === "block"}>{props.children}</Match>
      </Switch>
    </div>
  );
};

export default Tab;
