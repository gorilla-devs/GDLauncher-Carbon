import {
  getInstanceIdFromPath,
  isLibraryPath,
  setLastInstanceOpened
} from "@/utils/routes";
import { useNavigate } from "@solidjs/router";
import { JSX, createContext, createSignal, useContext } from "solid-js";

type NavigateOptions = {
  getLastInstance?: boolean;
  replace?: boolean;
};

type Context = (_path: string | number, _options?: NavigateOptions) => void;

const NavigationContext = createContext<Context>();

export const [lastPathVisited, setLastPathVisited] = createSignal("/");

export const NavigationManager = (props: { children: JSX.Element }) => {
  const navigate = useNavigate();

  const manager = (path: string | number, options?: NavigateOptions) => {
    console.log("NAVIGATE", path);

    if (typeof path == "number") {
      if ((document as any).startViewTransition) {
        (document as any).startViewTransition(() => {
          navigate(path);
        });
      } else {
        navigate(path);
      }
      return;
    }

    if (isLibraryPath(path) && options?.getLastInstance) {
      const parameters = path.split("?")[1];
      const instanceId = getInstanceIdFromPath(path);
      if (instanceId) setLastInstanceOpened(instanceId);
      if (instanceId === undefined) return;
      const route = `/library/${instanceId as string}/${parameters || ""}`;
      setLastPathVisited(route);
      if ((document as any).startViewTransition) {
        (document as any).startViewTransition(() => {
          navigate(route);
        });
      } else {
        navigate(route);
      }
    } else {
      setLastPathVisited(path);
      if ((document as any).startViewTransition) {
        (document as any).startViewTransition(() => {
          navigate(path, { replace: options?.replace });
        });
      } else {
        navigate(path, { replace: options?.replace });
      }
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
