import { JSXElement, Match, Switch } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element[] | JSXElement;
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  return (
    <div class="flex items-center h-auto bg-shade-8">
      <Switch>
        <Match when={tabsContext?.type === "underline"}>
          <div
            class="flex gap-6 border-b-shade-8 border-b-1 box-border overflow-auto"
            classList={{
              "flex-row": tabsContext?.orientation === "horizontal",
              "flex-col": tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.type === "block"}>
          <div
            class="flex items-center p-1 rounded-xl h-10 box-border overflow-auto"
            classList={{
              "flex-row": tabsContext?.orientation === "horizontal",
              "flex-col": tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export { TabList };
