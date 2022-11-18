import { useLocation } from "@solidjs/router";
import { createSignal, Show } from "solid-js";

const Sidebar = () => {
  const [opened, setOpened] = createSignal(true);
  const location = useLocation();

  return (
    <div
      style={{
        width: opened() ? "15rem" : "5rem",
        transition: "width .1s ease-in-out",
      }}
      class="h-full bg-black-black relative"
    >
      <div
        class="bg-[#272B35] absolute top-10 right-0 w-4 h-10 rounded-l-md flex justify-center items-center"
        onClick={() => setOpened(!opened())}
      >
        <Show
          when={opened()}
          fallback={
            <span class="i-ri:arrow-right-s-line text-[#404759] text-3xl" />
          }
        >
          <span class="i-ri:arrow-left-s-line text-[#404759] text-3xl" />
        </Show>
      </div>
    </div>
  );
};

export default Sidebar;
