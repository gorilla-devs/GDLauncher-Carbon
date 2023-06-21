import {
  Accessor,
  createContext,
  createSignal,
  JSX,
  useContext,
} from "solid-js";

type ContextMenuContextValue = {
  openMenu: Accessor<HTMLElement | null>;
  setOpenMenu: (_target: HTMLElement | null) => void;
  closeMenu: () => void;
};

const ContextMenuContext = createContext<ContextMenuContextValue>();

type ContextMenuProviderProps = {
  children: JSX.Element;
};

const ContextMenuProvider = (props: ContextMenuProviderProps) => {
  const [openMenu, setOpenMenu] = createSignal<HTMLElement | null>(null);

  const closeMenu = () => {
    setOpenMenu(null);
  };

  const value = { openMenu, setOpenMenu, closeMenu };

  return (
    <ContextMenuContext.Provider value={value}>
      {props.children}
    </ContextMenuContext.Provider>
  );
};

const useContextMenu = () => {
  return useContext(ContextMenuContext);
};

export { ContextMenuProvider, useContextMenu };
