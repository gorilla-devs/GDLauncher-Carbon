import { Show, createEffect, createSignal } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | string | number;
}

const TabPanel = (props: Props) => {
  const tabsContext = useTabsContext();

  const [index, setIndex] = createSignal(-1);

  const isTabPanelSelected = () => tabsContext?.isSelectedIndex(index());

  return (
    <div
      ref={(el: HTMLDivElement) => {
        setIndex(tabsContext?.registerTabPanel(el));
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

export default TabPanel;
