import { JSXElement, Show, createSignal } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | JSXElement | string | number | undefined;
}

const TabPanel = (props: Props) => {
  const tabsContext = useTabsContext();

  const [index, setIndex] = createSignal(-1);

  const isTabPanelSelected = () => tabsContext?.isSelectedIndex(index());

  return (
    <div
      ref={(el: HTMLDivElement) => {
        if (tabsContext) {
          setIndex(tabsContext.registerTabPanel(el));
        }
      }}
      class="w-full h-full"
      classList={{
        hidden: !isTabPanelSelected(),
      }}
    >
      <Show when={isTabPanelSelected()}>{props.children}</Show>
    </div>
  );
};

export { TabPanel };
