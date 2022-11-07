import { Component } from "solid-js";
import { Link, useRoutes, useLocation } from "@solidjs/router";
import { Transition } from "solid-transition-group";

import { routes } from "./routes";
import AppNavbar from "./components/AppNavbar";

const App: Component = () => {
  const location = useLocation();
  const Route = useRoutes(routes);

  return (
    <>
      <AppNavbar />
      <main class="relative">
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
                duration: 120,
              }
            );
            a.finished.then(done);
          }}
        >
          <Route />
        </Transition>
      </main>
    </>
  );
};

export default App;
