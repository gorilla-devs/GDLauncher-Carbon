import { Component, Show, Suspense } from "solid-js";
import { useRoutes } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
import { createInvalidateQuery } from "./utils/rspcClient";
import { Trans } from "@gd/i18n";
import initThemes from "./utils/theme";
import { useGDNavigate } from "./managers/NavigationManager";

const App: Component = () => {
  const Route = useRoutes(routes);
  const navigate = useGDNavigate();

  createInvalidateQuery();
  initThemes();

  return (
    <div class="relative w-screen">
      <Show when={process.env.NODE_ENV === "development"}>
        <div class="absolute gap-4 flex justify-center items-center cursor-pointer right-0 h-10 bottom-10 p-2 z-50 bg-light-600">
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
      <Suspense fallback={<></>}>
        <Route />
      </Suspense>
    </div>
  );
};

export default App;
