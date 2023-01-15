import { Component, createEffect, Show, Suspense } from "solid-js";
import { useRoutes, useNavigate } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
// import { Notifications } from "@gd/ui";
import { createInvalidateQuery, rspc } from "./utils/rspcClient";

const App: Component = () => {
  const Route = useRoutes(routes);
  const navigate = useNavigate();

  let javas = rspc.createQuery(() => ["java.getAvailableJavas", null]);

  createInvalidateQuery();

  const echoMsg = rspc.createQuery(() => ["echo", "something"]);
  // const subscription = rspc.createSubscription(() => ["subscriptions.pings"], {
  //   onData: (e) => console.log(e),
  // });

  createEffect(() => {
    console.log("pkgVersion", echoMsg.data);
    console.log("javas", javas.data);
  });

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
      <AppNavbar />
      <div class="flex h-screen w-screen z-10">
        <main class="relative flex-1 overflow-hidden">
          <Suspense fallback={<></>}>
            <Route />
          </Suspense>
        </main>
      </div>
      {/* <Notifications /> */}
    </div>
  );
};

export default App;
