import { Show, Suspense } from "solid-js";
import { useRoutes } from "@solidjs/router";
import { routes } from "./route";
import AppNavbar from "./components/Navbar";
import { Trans } from "@gd/i18n";
import initThemes from "./utils/theme";

type Props = {
  createInvalidateQuery: () => void;
};

const App = (props: Props) => {
  const Route = useRoutes(routes);

  // eslint-disable-next-line solid/reactivity
  props.createInvalidateQuery();
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
