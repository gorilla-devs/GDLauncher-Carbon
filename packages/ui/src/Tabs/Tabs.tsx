import { createContext, createSignal, JSXElement, useContext } from "solid-js";

export interface ITabsContext {
  type: string;
  orientation: string;
  // eslint-disable-next-line no-unused-vars
  setSelectedIndex: (index: number) => void;
  // eslint-disable-next-line no-unused-vars
  registerTab: (node: HTMLDivElement) => number;
  // eslint-disable-next-line no-unused-vars
  registerTabPanel: (node: HTMLDivElement) => number;
  // eslint-disable-next-line no-unused-vars
  isSelectedIndex: (index: number) => boolean;
}

export interface Props {
  children: Element[];
  defaultIndex?: number;
  // eslint-disable-next-line no-unused-vars
  onChange: (index: number) => void;
  type?: "underline" | "block";
  orientation?: "horizontal" | "vertical";
}

const TabsContext = createContext<ITabsContext>();

export function useTabsContext() {
  const context = useContext(TabsContext);

  if (!context) return;

  return context;
}

function Tabs(props: Props) {
  const [currentIndex, setCurrentIndex] = createSignal(props.defaultIndex ?? 0);
  const [tabs, setTabs] = createSignal<HTMLDivElement[]>([]);
  const [tabPanels, setTabPanels] = createSignal<HTMLDivElement[]>([]);

  const orientation = () => props.orientation || "horizontal";
  const type = () => props.type || "underline";

  const setSelectedIndex = (index: number) => {
    setCurrentIndex(index);
    props?.onChange?.(index);
  };

  const registerTab = (node: HTMLDivElement) => {
    const updatedArray = [...tabs(), node];
    setTabs(updatedArray);
    return updatedArray.length - 1;
  };

  const registerTabPanel = (node: HTMLDivElement) => {
    const updatedArray = [...tabPanels(), node];
    setTabPanels(updatedArray);
    return updatedArray.length - 1;
  };

  const isSelectedIndex = (index: number) => {
    return index === currentIndex();
  };

  const context = {
    isSelectedIndex,
    setSelectedIndex,
    registerTab,
    registerTabPanel,
    // getTabId,
    // getTabPanelId,
    // focusPrevTab,
    // focusNextTab,
    // focusFirstTab,
    // focusLastTab,
    type: type(),
    orientation: orientation(),
  };
  return (
    <TabsContext.Provider value={context}>
      {props.children}
    </TabsContext.Provider>
  );
}

export { Tabs };
