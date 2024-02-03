import HomeWithSidebar from "./HomeWithSidebar";
import HomeGrid from "./HomeGrid";
import { Show } from "solid-js";

const Home = () => {
  const gridLayout = () => true;

  return (
    <div>
      <Show when={!gridLayout()}>
        <HomeWithSidebar />
      </Show>
      <Show when={gridLayout()}>
        <HomeGrid />
      </Show>
    </div>
  );
};

export default Home;
