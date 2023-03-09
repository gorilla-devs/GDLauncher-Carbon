import { Component, Suspense } from "solid-js";
import { useRoutes } from "@solidjs/router";
import { routes } from "./route";
import { createInvalidateQuery } from "./utils/rspcClient";
import initThemes from "./utils/theme";

const App: Component = () => {
  const Route = useRoutes(routes);

  createInvalidateQuery();
  initThemes();

  return (
    <div class="relative w-screen">
      <Suspense fallback={<></>}>
        <Route />
      </Suspense>
    </div>
  );
};

export default App;
