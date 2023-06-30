import { createEffect, untrack } from "solid-js";
import { useLocation, useRoutes } from "@solidjs/router";
import { routes } from "./route";
import initThemes from "./utils/theme";
import { rspc } from "@/utils/rspcClient";
import { useModal } from "./managers/ModalsManager";
import { useKeyDownEvent } from "@solid-primitives/keyboard";

type Props = {
  createInvalidateQuery: () => void;
};

const App = (props: Props) => {
  const Route = useRoutes(routes);
  const modalsContext = useModal();
  const currentRoute = useLocation();

  // eslint-disable-next-line solid/reactivity
  props.createInvalidateQuery();

  initThemes();

  const isFirstRun = rspc.createQuery(() => ["settings.getSettings"]);
  const setIsFirstRun = rspc.createMutation(["settings.setSettings"]);

  createEffect(() => {
    if (isFirstRun.data?.isFirstLaunch && currentRoute.pathname !== "/") {
      untrack(() => {
        modalsContext?.openModal({ name: "onBoarding" });
        setIsFirstRun.mutate({ isFirstLaunch: false });
      });
    }
  });

  const event = useKeyDownEvent();

  createEffect(() => {
    // close modal clicking Escape
    const e = event();
    if (e) {
      if (e.key === "Escape") {
        untrack(() => {
          modalsContext?.closeModal();
        });
      }
    }
  });

  return (
    <div class="relative w-screen">
      <div class="w-screen flex z-10 h-auto">
        <main class="relative overflow-hidden flex-1">
          {/* <Suspense fallback={<></>}> */}
          <Route />
          {/* </Suspense> */}
        </main>
      </div>
    </div>
  );
};

export default App;
