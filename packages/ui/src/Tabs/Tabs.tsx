import {
  Accessor,
  JSXElement,
  createContext,
  createSignal,
  useContext,
  createEffect,
} from "solid-js";

export type SpacingTab = { ref: HTMLDivElement; type: string; space: number };
export type TabType = { ref: HTMLDivElement; type: string; ignored?: boolean };

type TabArrayElement = HTMLDivElement | SpacingTab | TabType;
export interface ITabsContext {
  gap?: number;
  paddingX?: string;
  paddingY?: string;
  variant: string;
  orientation: string;
  clearTabs: () => void;
  setSelectedIndex: (_: number) => void;
  registerTab: (_obj: TabType, _index?: number) => number;
  registerTabSpacing: (_obj: SpacingTab, _index?: number) => number;
  currentIndex: Accessor<number | undefined>;
  getRegisteredTabs: () => TabArrayElement[];
  registerTabPanel: (_: HTMLDivElement) => number;
  isSelectedIndex: (_: number) => boolean;
}

export interface Props {
  children: Element[] | JSXElement;
  defaultIndex?: number;
  index?: number;
  onChange?: (_: number) => void;
  variant?: "underline" | "block" | "traditional";
  orientation?: "horizontal" | "vertical";
  gap?: number;
  paddingX?: string;
  paddingY?: string;
}

const TabsContext = createContext<ITabsContext>();

export function useTabsContext() {
  const context = useContext(TabsContext);

  if (!context) return;

  return context;
}

function Tabs(props: Props) {
  const defaultIndex = () => props.defaultIndex ?? 0;
  const [currentIndex, setCurrentIndex] = createSignal(0);

  createEffect(() => {
    setCurrentIndex(props.index !== undefined ? props.index : defaultIndex());
  });

  const [tabs, setTabs] = createSignal<TabArrayElement[]>([]);
  const [tabPanels, setTabPanels] = createSignal<HTMLDivElement[]>([]);

  createEffect(() => {
    if (props.index !== undefined) {
      setCurrentIndex(props.index);
    }
  });

  const orientation = () => props.orientation || "horizontal";
  const variant = () => props.variant || "underline";
  const gap = () => props.gap;
  const paddingX = () => props?.paddingX;
  const paddingY = () => props?.paddingY;

  const setSelectedIndex = (index: number) => {
    setCurrentIndex(index);
    props?.onChange?.(index);
  };

  const registerTab = (obj: TabType, index?: number) => {
    if (index !== undefined) {
      const updatedArray = [...tabs()];
      updatedArray[index] = obj;
      setTabs(updatedArray);
      return index;
    }
    const updatedArray = [...tabs(), obj] as TabArrayElement[];
    setTabs(updatedArray);
    return updatedArray.length - 1;
  };

  const registerTabSpacing = (obj: SpacingTab, index?: number) => {
    if (index !== undefined) {
      const updatedArray = [...tabs()];
      updatedArray[index] = obj;
      setTabs(updatedArray);
      return index;
    }
    const updatedArray = [...tabs(), obj] as TabArrayElement[];

    setTabs(updatedArray);
    return updatedArray.length - 1;
  };

  const getRegisteredTabs = () => {
    return tabs();
  };

  const registerTabPanel = (node: HTMLDivElement) => {
    const updatedArray = [...tabPanels(), node];
    setTabPanels(updatedArray);
    return updatedArray.length - 1;
  };

  const isSelectedIndex = (index: number) => {
    return index === currentIndex();
  };

  const clearTabs = () => {
    return setTabs([]);
  };

  const context = {
    isSelectedIndex,
    setSelectedIndex,
    clearTabs,
    registerTab,
    currentIndex,
    getRegisteredTabs,
    registerTabSpacing,
    registerTabPanel,
    gap: gap(),
    variant: variant(),
    orientation: orientation(),
    paddingX: paddingX(),
    paddingY: paddingY(),
  };

  return (
    <TabsContext.Provider value={context}>
      <div
        class="flex transition-all duration-100 ease-in-out w-full max-h-full"
        classList={{
          "flex-row": orientation() === "vertical",
          "flex-col": orientation() === "horizontal",
        }}
      >
        {props.children}
      </div>
    </TabsContext.Provider>
  );
}

export { Tabs };
