import { JSXElement, Show, createSignal, createEffect } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element | JSXElement | string | number | undefined;
}

const TabPanel = (props: Props) => {
  const tabsContext = useTabsContext();
  const [ref, setRef] = createSignal<HTMLDivElement>();
  const [index, setIndex] = createSignal(-1);

  const isTabPanelSelected = () => tabsContext?.isSelectedIndex(index());

  let prevWidth: number | undefined = undefined;
  createEffect(() => {
    if (tabsContext) {
      if (ref() && ref()!.offsetWidth !== prevWidth) {
        prevWidth = ref()!.offsetWidth;
        setIndex(tabsContext.registerTabPanel(ref()!));
      }
    }
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
