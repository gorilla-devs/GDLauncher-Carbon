import { Component, createEffect, createSignal, onMount } from "solid-js";
import { Link, useRoutes, useLocation } from "@solidjs/router";
import { Transition } from "solid-transition-group";

import { routes } from "./routes";

interface LocationChange {
  prev: string | undefined;
  next: string | undefined;
}

const App: Component = () => {
  const [locationChange, setLocationChange] = createSignal<LocationChange>({
    prev: undefined,
    next: undefined,
  });
  const location = useLocation();
  const Route = useRoutes(routes);

  createEffect((prev: string | undefined) => {
    setLocationChange({ prev, next: location.pathname });
    return location.pathname;
  });

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
              value={location.pathname}
            />
          </li>
        </ul>
      </nav>

      <main class="relative">
        <Transition
          onEnter={(el, done) => {
            const { prev, next } = locationChange();
            const prevIndex = routes.findIndex((v) => v.path === prev);
            const nextIndex = routes.findIndex((v) => v.path === next);
            const isForward = nextIndex > prevIndex;

            let transformation;
            if (
              prev === undefined ||
              next === undefined ||
              prevIndex === -1 ||
              nextIndex === -1
            ) {
              transformation = [{ opacity: 0 }, { opacity: 1 }];
            } else if (isForward) {
              transformation = [
                {
                  transform: "translateX(100%)",
                },
                {
                  transform: "translateX(0%)",
                },
              ];
            } else {
              transformation = [
                {
                  transform: "translateX(-100%)",
                },
                {
                  transform: "translateX(0%)",
                },
              ];
            }

            console.log("ENTER", prevIndex, prev, nextIndex, next);

            const a = el.animate(transformation, {
              duration: 250,
            });
            a.finished.then(done);
          }}
          onExit={(el, done) => {
            const { prev, next } = locationChange();
            const prevIndex = routes.findIndex((v) => v.path === prev);
            const nextIndex = routes.findIndex((v) => v.path === next);
            const isForward = nextIndex < prevIndex;

            let transformation;
            if (
              prev === undefined ||
              next === undefined ||
              prevIndex === -1 ||
              nextIndex === -1
            ) {
              transformation = [{ opacity: 1 }, { opacity: 0 }];
            } else if (isForward) {
              transformation = [
                {
                  transform: "translateX(0%)",
                },
                {
                  transform: "translateX(-100%)",
                },
              ];
            } else {
              transformation = [
                {
                  transform: "translateX(0%)",
                },
                {
                  transform: "translateX(100%)",
                },
              ];
            }

            console.log("EXIT", prevIndex, prev, nextIndex, next);

            const a = el.animate(transformation, {
              duration: 250,
            });
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
