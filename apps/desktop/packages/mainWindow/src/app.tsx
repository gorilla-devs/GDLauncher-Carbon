import { Component, onMount, Show, Suspense } from "solid-js";
import { useRoutes, useNavigate } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
import initThemes from "@/utils/theme";
import { Trans } from "@gd/i18n";

const App: Component = () => {
  const Route = useRoutes(routes);
  const navigate = useNavigate();
  onMount(() => {
    // TODO: maybe wait for initThemes to finish before clearing loading
    initThemes();
    window.clearLoading();
  });

  return (
    <div class="relative w-screen h-screen">
      <Show when={process.env.NODE_ENV === "development"}>
        <div class="absolute bottom-10 right-0 h-10 p-2 gap-4 z-50 bg-light-600 flex justify-center items-center cursor-pointer">
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
      <div class="flex h-screen w-screen z-10">
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
