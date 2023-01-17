import { JSXElement, Match, Show, Switch } from "solid-js";
import { SpacingTab, TabType, useTabsContext } from "./Tabs";

interface Props {
  aligment?: "between" | "default";
  children: Element[] | JSXElement;
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  const tabs = () => tabsContext?.getRegisteredTabs() || [];

  const currentIndex = () => tabsContext?.currentIndex() || 0;
  const currentTab = () => tabs()[currentIndex()];

  const isIgnored = () => (currentTab() as TabType)?.ignored;

  const getPositionPx = (index: number) => {
    const filteredTabs = tabs()?.slice(0, index);
    const gap = tabsContext?.gap ?? 24;

    if (index < 0 || index > tabs()?.length) return 0;

    let dimension = 0;
    for (const tab of filteredTabs) {
      const isSpacing =
        typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

      if (tabsContext?.orientation === "horizontal") {
        if (isSpacing) {
          if (isSpacing) dimension += (tab as SpacingTab).space + gap;
        } else {
          dimension += (tab as TabType).ref.offsetWidth + gap;
        }
      } else {
        if (isSpacing) {
          if (isSpacing) dimension += (tab as SpacingTab).space + gap;
        } else {
          dimension += (tab as TabType).ref.offsetHeight + gap;
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

    return isSpacing ? "auto" : (tab as TabType).ref.offsetWidth;
  };

  const getHeight = (index: number) => {
    if (index < 0 || index > tabs()?.length) return 0;

    const tab = tabs()[index];

    const isSpacing =
      typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

    return isSpacing ? "auto" : (tab as TabType).ref.offsetHeight;
  };

  return (
    <div
      class="flex relative items-center h-auto w-full"
      classList={{
        "bg-shade-8": tabsContext?.variant === "underline",
        "bg-shade-9": tabsContext?.variant === "block",
      }}
    >
      <Switch>
        <Match when={tabsContext?.variant === "underline"}>
          <div
            class="flex border-b-shade-8 border-b-1 box-border overflow-auto w-full"
            classList={{
              "gap-6": tabsContext?.orientation !== undefined,
              "flex-row": tabsContext?.orientation === "horizontal",
              "flex-col": tabsContext?.orientation === "vertical",
              "justify-between": props.aligment === "between",
            }}
            style={{
              gap: tabsContext?.gap?.toString(),
            }}
          >
            {props.children}
            <Show when={tabs()[currentIndex()] && !isIgnored()}>
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
            class="flex items-center p-2 rounded-xl box-border overflow-auto w-full"
            classList={{
              "gap-6": tabsContext?.orientation !== undefined,
              "flex-row": tabsContext?.orientation === "horizontal",
              "flex-col": tabsContext?.orientation === "vertical",
              "justify-between": props.aligment === "between",
            }}
            style={{
              gap: tabsContext?.gap?.toString(),
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
