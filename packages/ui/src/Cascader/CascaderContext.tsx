import {
  Accessor,
  createContext,
  createSignal,
  JSX,
  useContext,
} from "solid-js";

type ContextCascaderContextValue = {
  openCascader: Accessor<HTMLElement | null>;
  setOpenCascader: (_target: HTMLElement | null) => void;
  closeCascader: () => void;
};

const ContextCascaderContext = createContext<ContextCascaderContextValue>();

type ContextCascaderProviderProps = {
  children: JSX.Element;
};

const ContextCascaderProvider = (props: ContextCascaderProviderProps) => {
  const [openCascader, setOpenCascader] = createSignal<HTMLElement | null>(
    null
  );

  const closeCascader = () => {
    setOpenCascader(null);
  };
  const value = { openCascader, setOpenCascader, closeCascader };

  return (
    <ContextCascaderContext.Provider value={value}>
      {props.children}
    </ContextCascaderContext.Provider>
  );
};

const useContextCascader = () => {
  return useContext(ContextCascaderContext);
};

export { ContextCascaderProvider, useContextCascader };

export const Exported = 5;
