import { JSXElement, Match, Show, Switch } from "solid-js";
import { SpacingTab, TabType, useTabsContext } from "./Tabs";

interface Props {
  aligment?: "between" | "default";
  children: Element[] | JSXElement;
  heightClass?: string;
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  const tabs = () => tabsContext?.getRegisteredTabs() || [];

  const currentIndex = () => tabsContext?.currentIndex() || 0;
  const currentTab = () => tabs()[currentIndex()];

  const isIgnored = () => (currentTab() as TabType)?.ignored;

  const getPositionPx = (index: number) => {
    const filteredTabs = tabs()?.slice(0, index);
    if (index < 0 || index > tabs()?.length) return 0;

    let dimension = 0;
    for (const tab of filteredTabs) {
      const isSpacing =
        typeof tab === "object" && (tab as SpacingTab)?.type === "spacing";

      if (tabsContext?.orientation() === "horizontal") {
        if (isSpacing) {
          if (isSpacing) dimension += (tab as SpacingTab).space;
        } else {
          dimension += (tab as TabType).ref.offsetWidth;
        }
      } else {
        if (isSpacing) {
          if (isSpacing) dimension += (tab as SpacingTab).space;
        } else {
          dimension += (tab as TabType).ref.offsetHeight;
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
      class={`flex relative items-start w-full min-h-12 ${
        props.heightClass ?? "h-full"
      }`}
      classList={{
        "bg-darkSlate-800": tabsContext?.variant() === "underline",
        "bg-darkSlate-900": tabsContext?.variant() === "block",
      }}
    >
      <Switch>
        <Match when={tabsContext?.variant() === "underline"}>
          <div
            class="flex border-b-darkSlate-800 border-b-1 box-border overflow-auto w-full h-full"
            classList={{
              "gap-6": tabsContext?.orientation() !== undefined,
              "flex-row": tabsContext?.orientation() === "horizontal",
              "flex-col": tabsContext?.orientation() === "vertical",
              "justify-between": props.aligment === "between",
            }}
            style={{
              gap: tabsContext?.gap?.()?.toString() + "rem",
            }}
          >
            {props.children}
            <Show when={tabs()[currentIndex()] && !isIgnored()}>
              <div
                class="absolute bottom-1 h-1 bg-primary-500"
                classList={{
                  "top-0 w-1 right-0":
                    tabsContext?.orientation() === "vertical",
                  "left-0": tabsContext?.orientation() === "horizontal",
                }}
                style={{
                  ...(tabsContext?.orientation() === "horizontal"
                    ? {
                        width: `${getWidth(currentIndex())}px`,
                      }
                    : {
                        height: `${getHeight(currentIndex())}px`,
                      }),
                  ...(tabsContext?.orientation() === "horizontal"
                    ? {
                        transform: `translateX(calc(${getPositionPx(
                          currentIndex()
                        )}px + (${
                          tabsContext?.gap?.() ?? 1.5
                        }rem * ${currentIndex()})))`,
                      }
                    : {
                        transform: `translateY(calc(${getPositionPx(
                          currentIndex()
                        )}px+ (${
                          tabsContext?.gap?.() ?? 1.5
                        }rem * ${currentIndex()})))`,
                      }),
                }}
              />
            </Show>
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "block"}>
          <div
            class="flex items-center m-2 rounded-xl box-border overflow-auto w-full"
            classList={{
              "gap-6": tabsContext?.orientation() !== undefined,
              "flex-row": tabsContext?.orientation() === "horizontal",
              "flex-col": tabsContext?.orientation() === "vertical",
              "justify-between": props.aligment === "between",
            }}
            style={{
              gap: (tabsContext?.gap?.()?.toString() ?? 1.5) + "rem",
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "traditional"}>
          <div
            class="flex items-center box-border overflow-auto w-full scrollbar-hide"
            classList={{
              "flex-row": tabsContext?.orientation() === "horizontal",
              "flex-col": tabsContext?.orientation() === "vertical",
              "justify-between": props.aligment === "between",
            }}
            style={{
              gap: (tabsContext?.gap?.()?.toString() ?? 1.5) + "rem",
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
