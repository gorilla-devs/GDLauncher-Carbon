import {
  getInstanceIdFromPath,
  isLibraryPath,
  lastInstanceOpened,
  setLastInstanceOpened,
} from "@/utils/routes";
import { useNavigate } from "@solidjs/router";
import { JSX, createContext, useContext } from "solid-js";

type Context = {
  navigate: (_path: string) => void;
};
const NavigationContext = createContext<Context>();

export const NavigationManager = (props: { children: JSX.Element }) => {
  const navigate = useNavigate();
  const manager = {
    navigate: (path: string) => {
      if (isLibraryPath(path)) {
        const parameters = path.split("?")[1];
        const instanceId = getInstanceIdFromPath(path);
        if (instanceId) setLastInstanceOpened(instanceId);
        navigate(`/library/${lastInstanceOpened()}/${parameters || ""}`);
      } else navigate(path);
    },
  };

  return (
    <NavigationContext.Provider value={manager}>
      {props.children}
    </NavigationContext.Provider>
  );
};

export const useGdNavigation = () => {
  return useContext(NavigationContext);
};
