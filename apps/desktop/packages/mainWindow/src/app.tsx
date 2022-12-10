import { Component, createSignal, Show } from "solid-js";
import { useRoutes, useLocation, useNavigate } from "@solidjs/router";
import { routes } from "./routes";
import AppNavbar from "./components/Navbar";
import Notifications from "./notificationManager";

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
        <main class="relative flex-1 overflow-hidden">
          <Route />
        </main>
      </div>
      <Notifications />
    </div>
  );
};

export default App;
