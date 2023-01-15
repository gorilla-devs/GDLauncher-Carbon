import { JSXElement, Match, Show, Switch, createEffect } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element[] | JSXElement;
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  const tabs = () => tabsContext?.getRegisteredTabs() || [];

  const currentIndex = () => tabsContext?.currentIndex() || 0;

  const getPositionPx = (index: number) => {
    const filteredTabs = tabs()?.slice(0, index);

    if (index < 0 || index > tabs()?.length) return 0;

    let dimension = 0;
    for (const tab of filteredTabs) {
      if (tabsContext?.orientation === "horizontal") {
        dimension += tab.offsetWidth + 24;
      } else dimension += tab.offsetHeight + 24;
    }
    return dimension;
  };

  const getWidth = (index: number) => {
    if (index < 0 || index > tabs()?.length) return 0;

    const tab = tabs()[index];
    return tab?.offsetWidth;
  };

  const getHeight = (index: number) => {
    if (index < 0 || index > tabs()?.length) return 0;

    const tab = tabs()[index];
    return tab?.offsetHeight;
  };

  createEffect(() => {
    console.log(
      "TEST",
      getPositionPx(currentIndex()),
      tabs()[currentIndex()],
      tabs()
    );
  });

  return (
    <div
      class="flex relative items-center h-auto"
      classList={{
        "bg-shade-8": tabsContext?.variant === "underline",
        "bg-shade-9": tabsContext?.variant === "block",
      }}
    >
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
            <Show when={tabs()[currentIndex()]}>
              <div
                class="absolute bottom-1 h-1 bg-primary transition-all duration-100 ease-in-out animate-fade-in"
                classList={{
                  "top-0 w-1 right-0": tabsContext?.orientation === "vertical",
                  "left-0": tabsContext?.orientation === "horizontal",
                }}
                style={{
                  ...(tabsContext?.orientation === "horizontal"
                    ? {
                        width: `${getWidth(currentIndex())}px`,
                      }
                    : {
                        height: `${getHeight(currentIndex())}px`,
                      }),
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
            </Show>
          </div>
        </Match>
        <Match when={tabsContext?.variant === "block"}>
          <div
            class="flex gap-6 items-center p-2 rounded-xl box-border overflow-auto"
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
