import { JSXElement, Match, Show, Switch } from "solid-js";
import { SpacingTab, useTabsContext } from "./Tabs";

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
      const isSpacing =
        typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

      if (tabsContext?.orientation === "horizontal") {
        if (isSpacing) {
          dimension += (tab as SpacingTab).space + 24;
        } else {
          dimension += (tab as HTMLDivElement).offsetWidth + 24;
        }
      } else {
        if (isSpacing) {
          dimension += (tab as SpacingTab).space + 24;
        } else {
          dimension += (tab as HTMLDivElement).offsetHeight + 24;
        }
      }
    }
    return dimension;
  };

  const getWidth = (index: number) => {
    if (index < 0 || index > tabs()?.length) return 0;

    const tab = tabs()[index];

    const isSpacing =
      typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

    return isSpacing ? "auto" : (tab as HTMLDivElement)?.offsetWidth;
  };

  const getHeight = (index: number) => {
    if (index < 0 || index > tabs()?.length) return 0;

    const tab = tabs()[index];

    const isSpacing =
      typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

    return isSpacing ? "auto" : (tab as HTMLDivElement)?.offsetHeight;
  };

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
                class="absolute bottom-1 h-1 bg-primary transition-all duration-100 ease-in-out"
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
