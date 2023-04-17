import {
  getInstanceIdFromPath,
  isLibraryPath,
  lastInstanceOpened,
  setLastInstanceOpened,
} from "@/utils/routes";
import { useNavigate } from "@solidjs/router";
import { JSX, createContext, createSignal, useContext } from "solid-js";

type NavigateOptions = {
  getLastInstance?: boolean;
};

type Context = (_path: string, _options?: NavigateOptions) => void;

const NavigationContext = createContext<Context>();

export const [lastPathVisited, setLastPathVisited] = createSignal("/");

export const NavigationManager = (props: { children: JSX.Element }) => {
  const navigate = useNavigate();
  const manager = (path: string, options?: NavigateOptions) => {
    if (isLibraryPath(path) && options?.getLastInstance) {
      const parameters = path.split("?")[1];
      const instanceId = getInstanceIdFromPath(path);
      if (instanceId) setLastInstanceOpened(instanceId);

      const route = `/library/${lastInstanceOpened()}/${parameters || ""}`;
      setLastPathVisited(route);
      navigate(route);
    } else {
      setLastPathVisited(path);
      navigate(path);
    }
  };

  return (
    <NavigationContext.Provider value={manager}>
      {props.children}
    </NavigationContext.Provider>
  );
};

export const useGDNavigate = (): Context => {
  return useContext(NavigationContext) as Context;
};
