import { Show, Suspense } from "solid-js";
import { useBeforeLeave, useRoutes } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
import { Trans } from "@gd/i18n";
import initThemes from "./utils/theme";
import { useGDNavigate } from "./managers/NavigationManager";
import { rspc } from "./utils/rspcClient";

type Props = {
  createInvalidateQuery: () => void;
};

const App = (props: Props) => {
  const Route = useRoutes(routes);
  const navigate = useGDNavigate();

  // eslint-disable-next-line solid/reactivity
  props.createInvalidateQuery();

  initThemes();

  let data = rspc.createQuery(() => ["account.getActiveUuid"]);
  useBeforeLeave(() => {
    if (!data.data) navigate("/");
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
              key="login"
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
              key="logout"
              options={{
                defaultValue: "logout",
              }}
            />
          </div>
        </div>
      </Show>
      <AppNavbar />
      <div class="flex w-screen h-auto z-10">
        <main class="relative overflow-hidden flex-1">
          <Suspense fallback={<></>}>
            <Route />
          </Suspense>
        </main>
      </div>
    </div>
  );
};

export default App;
