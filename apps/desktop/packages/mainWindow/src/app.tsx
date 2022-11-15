import { Component, Show } from "solid-js";
import { useRoutes, useLocation } from "@solidjs/router";
import { Transition } from "solid-transition-group";
import { Pattern } from "@gd/ui";
import { routes } from "./routes";
import AppNavbar from "./components/AppNavbar";
import Sidebar from "./components/Sidebar";
import { AdsBanner } from "./components/AdBanner";
import Notifications from "./notificationManager";

const App: Component = () => {
  const location = useLocation();
  const Route = useRoutes(routes);

  return (
    <div class="relative w-screen h-screen">
      <AppNavbar />
      <div class="flex h-screen w-screen z-10">
        <Show when={location.pathname !== "/"}>
          <Sidebar />
        </Show>
        <main class="relative flex-1">
          <Transition
            onEnter={(el, done) => {
              const a = el.animate(
                [
                  {
                    opacity: 0,
                  },
                  {
                    opacity: 1,
                  },
                ],
                {
                  duration: 120,
                }
              );
              a.finished.then(done);
            }}
            onAfterEnter={(el) => {
              el.classList.remove("opacity-0");
            }}
            onExit={(el, done) => {
              const a = el.animate(
                [
                  {
                    opacity: 1,
                  },
                  {
                    opacity: 0,
                  },
                ],
                {
                  duration: 0,
                }
              );
              a.finished.then(done);
            }}
          >
            <Route />
          </Transition>
        </main>
        <Show when={location.pathname !== "/"}>
          <AdsBanner />
        </Show>
        <Show when={location.pathname !== "/"}>
          <Pattern class="absolute top-0 left-0 right-0 bottom-0" />
        </Show>
      </div>
      <Notifications />
    </div>
  );
};

export default App;
