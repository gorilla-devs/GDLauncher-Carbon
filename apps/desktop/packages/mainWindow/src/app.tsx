import { Component, Show, Suspense } from "solid-js";
import { useRoutes, useNavigate } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
import { createInvalidateQuery } from "./utils/rspcClient";
import { Trans } from "@gd/i18n";
import initThemes from "./utils/theme";

const App: Component = () => {
  const Route = useRoutes(routes);
  const navigate = useNavigate();

  createInvalidateQuery();
  initThemes();

  return (
    <div class="w-screen relative">
      <Show when={process.env.NODE_ENV === "development"}>
        <div class="absolute gap-4 flex justify-center items-center cursor-pointer bottom-10 right-0 h-10 p-2 z-50 bg-light-600">
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
        <main class="relative flex-1 overflow-hidden">
          <Suspense fallback={<></>}>
            <Route />
          </Suspense>
        </main>
      </div>
    </div>
  );
};

export default App;
