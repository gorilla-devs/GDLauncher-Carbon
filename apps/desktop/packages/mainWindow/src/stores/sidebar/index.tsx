import { createSignal } from "solid-js";

const [isSidebarOpened, setIsSidebarOpened] = createSignal(true);

export const toggleSidebar = () => {
  setIsSidebarOpened(!isSidebarOpened());
};

export { isSidebarOpened };
