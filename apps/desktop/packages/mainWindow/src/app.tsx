import { Component, createSignal, Show } from "solid-js";
import { useRoutes, useLocation, useNavigate } from "@solidjs/router";
import { Transition } from "solid-transition-group";
import { Pattern } from "@gd/ui";
import { routes } from "./routes";
import AppNavbar from "./components/AppNavbar";
import Sidebar from "./components/Sidebar";
import { AdsBanner } from "./components/AdBanner";
import { Notifications } from "@gd/ui";

const App: Component = () => {
  const location = useLocation();
  const Route = useRoutes(routes);
  const navigate = useNavigate();
  const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false);

  return (
    <div class="relative w-screen h-screen">
      <Show when={process.env.NODE_ENV === "development"}>
        <div class="absolute top-0 right-0 h-10 p-2 gap-4 z-50 bg-light-600 flex justify-center items-center cursor-pointer">
          <div
            onClick={() => {
              navigate("/library");
            }}
          >
            LOGIN
          </div>
          <div
            onClick={() => {
              navigate("/");
            }}
          >
            LOGOUT
          </div>
        </div>
      </Show>
      <AppNavbar sidebarCollapsed={sidebarCollapsed()} />
      <div class="flex h-screen w-screen z-10">
        <Show when={location.pathname !== "/"}>
          <Sidebar
            collapsed={sidebarCollapsed()}
            setSidebarCollapsed={setSidebarCollapsed}
          />
        </Show>
        <main class="relative flex-1 overflow-hidden">
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
