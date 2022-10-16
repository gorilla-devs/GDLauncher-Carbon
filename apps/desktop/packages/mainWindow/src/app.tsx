import { Component } from "solid-js";
import { Link, useRoutes, useLocation } from "@solidjs/router";
import { Transition } from "solid-transition-group";

import { routes } from "./routes";

const App: Component = () => {
  const location = useLocation();
  const Route = useRoutes(routes);

  return (
    <>
      <nav class="bg-gray-200 text-gray-900 px-4">
        <ul class="flex items-center">
          <li class="py-2 px-4">
            <Link href="/" class="no-underline hover:underline">
              Home
            </Link>
          </li>
          <li class="py-2 px-4">
            <Link href="/about" class="no-underline hover:underline">
              About
            </Link>
          </li>
          <li class="py-2 px-4">
            <Link href="/error" class="no-underline hover:underline">
              Error
            </Link>
          </li>

          <li class="text-sm flex items-center space-x-1 ml-auto">
            <span>URL:</span>
            <input
              class="w-75px p-1 bg-white text-sm rounded-lg"
              type="text"
              readOnly
              value={location.pathname + location.search}
            />
          </li>
        </ul>
      </nav>

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
