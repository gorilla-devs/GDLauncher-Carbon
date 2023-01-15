import {
  Accessor,
  JSXElement,
  createContext,
  createSignal,
  useContext,
  createEffect,
} from "solid-js";

export interface ITabsContext {
  variant: string;
  orientation: string;
  setSelectedIndex: (_: number) => void;
  registerTab: (_: HTMLDivElement) => number;
  currentIndex: Accessor<number | undefined>;
  getRegisteredTabs: () => HTMLDivElement[];
  registerTabPanel: (_: HTMLDivElement) => number;
  isSelectedIndex: (_: number) => boolean;
}

export interface Props {
  children: Element[] | JSXElement;
  defaultIndex?: number;
  index?: number;
  onChange?: (_: number) => void;
  variant?: "underline" | "block";
  orientation?: "horizontal" | "vertical";
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

  const [tabs, setTabs] = createSignal<HTMLDivElement[]>([]);
  const [tabPanels, setTabPanels] = createSignal<HTMLDivElement[]>([]);

  createEffect(() => {
    if (props.index !== undefined) {
      setCurrentIndex(props.index);
    }
  });

  const orientation = () => props.orientation || "horizontal";
  const variant = () => props.variant || "underline";

  const setSelectedIndex = (index: number) => {
    setCurrentIndex(index);
    props?.onChange?.(index);
  };

  const registerTab = (node: HTMLDivElement) => {
    const updatedArray = [...tabs(), node];
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

  const context = {
    isSelectedIndex,
    setSelectedIndex,
    registerTab,
    currentIndex,
    getRegisteredTabs,
    registerTabPanel,
    variant: variant(),
    orientation: orientation(),
  };

  return (
    <TabsContext.Provider value={context}>
      <div
        class="flex transition-all duration-100 ease-in-out"
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
