import { createEffect, JSXElement, Match, Switch } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element[] | JSXElement;
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  const currentIndex = () => tabsContext?.currentIndex() || 0;

  const getPositionPx = (index: number) => {
    const tabs = tabsContext?.getRegisteredTabs();
    const filteredTabs = tabs?.slice(0, index) || [];

    let dimension = 0;
    for (const tab of filteredTabs) {
      if (tabsContext?.orientation === "horizontal") {
        dimension += tab.offsetWidth + 24;
      } else dimension += tab.offsetHeight + 24;
    }
    return dimension;
  };

  return (
    <div class="flex relative items-center h-auto bg-shade-8">
      <Switch>
        <Match when={tabsContext?.variant === "underline"}>
          <div
            class="flex gap-6 border-b-shade-8 border-b-1 box-border overflow-auto"
            classList={{
              "flex-row": tabsContext?.orientation === "horizontal",
              "flex-col": tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
            <div
              class="absolute left-0 right-0 bottom-1 h-1 bg-primary transition-transform duration-100 ease-in-out"
              style={{
                width: `${
                  tabsContext?.getRegisteredTabs()[currentIndex()]?.offsetWidth
                }px`,
                ...(tabsContext?.orientation === "horizontal"
                  ? {
                      transform: `translateX(${getPositionPx(
                        currentIndex()
                      )}px)`,
                    }
                  : {
                      transform: `translateY(${getPositionPx(
                        currentIndex()
                      )}px)`,
                    }),
              }}
            />
          </div>
        </Match>
        <Match when={tabsContext?.variant === "block"}>
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
