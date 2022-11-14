import { createSignal } from "solid-js";

const Sidebar = () => {
  const [opened, setOpened] = createSignal(true);

  return (
    <div
      style={{
        width: opened() ? "15rem" : "5rem",
        transition: "width .1s ease-in-out",
      }}
      class="h-full bg-black-black relative"
    >
      <div
        class="bg-[#272B35] absolute top-10 right-0 w-4 h-10 rounded-l-md"
        onClick={() => setOpened(!opened())}
      >
        <span class="i-custom:circle"></span>
      </div>
    </div>
  );
};

export default Sidebar;
