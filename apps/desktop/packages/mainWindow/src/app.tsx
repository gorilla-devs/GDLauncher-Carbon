import { createEffect, untrack } from "solid-js";
import { useLocation, useRoutes } from "@solidjs/router";
import { routes } from "./route";
import initThemes from "./utils/theme";
import { rspc } from "@/utils/rspcClient";
import { useModal } from "./managers/ModalsManager";
import { useKeyDownEvent } from "@solid-primitives/keyboard";
import initAnalytics from "@/utils/analytics";
import { checkForUpdates } from "./utils/updater";

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

  const isFirstRun = rspc.createQuery(() => ["settings.getSettings"], {
    onSuccess(data) {
      checkForUpdates(data.releaseChannel);

      if (data.metricsLevel !== 0 && data.metricsLevel !== null) {
        initAnalytics(data.metricsLevel);
      }
    },
  });

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
    <div class="relative w-screen select-none">
      <div class="w-screen flex z-10 h-auto">
        <main class="relative flex-1">
          {/* <Suspense fallback={<></>}> */}
          <Route />
          {/* </Suspense> */}
        </main>
      </div>
    </div>
  );
};

export default App;
