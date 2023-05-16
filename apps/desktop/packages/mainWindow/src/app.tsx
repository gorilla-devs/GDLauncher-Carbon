import { Show, createEffect, untrack } from "solid-js";
import { useLocation, useRoutes } from "@solidjs/router";
import { routes } from "./route";
import { Trans } from "@gd/i18n";
import initThemes from "./utils/theme";
import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "./managers/NavigationManager";
import { useModal } from "./managers/ModalsManager";

type Props = {
  createInvalidateQuery: () => void;
};

const App = (props: Props) => {
  const Route = useRoutes(routes);
  const navigate = useGDNavigate();
  const modalsContext = useModal();
  const currentRoute = useLocation();

  // eslint-disable-next-line solid/reactivity
  props.createInvalidateQuery();

  initThemes();

  const isFirstRun = rspc.createQuery(() => ["settings.getIsFirstLaunch"]);
  const setIsFirstRun = rspc.createMutation(["settings.setIsFirstLaunch"]);

  createEffect(() => {
    if (isFirstRun.data && currentRoute.pathname !== "/") {
      untrack(() => {
        modalsContext?.openModal({ name: "onBoarding" });
        setIsFirstRun.mutate(false);
      });
    }
  });

  return (
    <div class="w-screen relative">
      <Show when={process.env.NODE_ENV === "development"}>
        <div class="absolute gap-4 flex justify-center items-center cursor-pointer h-10 bottom-10 right-0 p-2 z-50 bg-light-600">
          <div
            onClick={() => {
              navigate("/library");
            }}
          >
            <Trans
              key="login.login"
              options={{
                defaultValue: "login",
              }}
            />
          </div>
          <div
            onClick={() => {
              // deleteMutation.mutate();
              navigate("/");
            }}
          >
            <Trans
              key="login.logout"
              options={{
                defaultValue: "logout",
              }}
            />
          </div>
        </div>
      </Show>
      <div class="flex w-screen h-auto z-10">
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
