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
  // const deleteMutation = rspc.createMutation(["account.deleteAccount"]);

  createInvalidateQuery();
  initThemes();

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
