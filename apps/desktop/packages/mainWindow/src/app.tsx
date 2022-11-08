import { Component, Show } from "solid-js";
import { Link, useRoutes, useLocation } from "@solidjs/router";
import { Transition } from "solid-transition-group";

import { routes } from "./routes";
import AppNavbar from "./components/AppNavbar";
import Sidebar from "./components/Sidebar";
import { AdsBanner } from "./components/AdBanner";

const App: Component = () => {
  const location = useLocation();
  const Route = useRoutes(routes);

  return (
    <div class="relative w-screen h-screen">
      <AppNavbar />
      <div class="flex h-screen w-screen">
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
      </div>
    </div>
  );
};

export default App;
