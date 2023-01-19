import { JSXElement, Show, createSignal, onMount } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | JSXElement | string | number | undefined;
}

const TabPanel = (props: Props) => {
  const tabsContext = useTabsContext();
  const [ref, setRef] = createSignal<HTMLDivElement>();
  const [index, setIndex] = createSignal(-1);

  const isTabPanelSelected = () => tabsContext?.isSelectedIndex(index());

  onMount(() => {
    // eslint-disable-next-line solid/reactivity
    queueMicrotask(() => {
      if (tabsContext) {
        setIndex(tabsContext.registerTabPanel(ref()!));
      }
    });
  });

  return (
    <div
      ref={setRef}
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
