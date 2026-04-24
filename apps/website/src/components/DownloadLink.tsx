import { onMount, createSignal } from "solid-js";
import { detectOS } from "../utils/detectOS";
import type { OS } from "../utils/detectOS";

const OS_ICONS: Record<OS, string> = {
  Windows: "i-simple-icons:windows11",
  MacOS: "i-simple-icons:apple",
  Linux: "i-simple-icons:linux",
}

const OS_URL: Record<OS, string> = {
  Windows: "/download/windows",
  MacOS: "/download/mac",
  Linux: "/download/linux",
}

export const DownloadLink = () => {
  const [os, setOS] = createSignal<OS>("Windows");

  onMount(() => {
    setOS(detectOS());
  });

  return (
    <a href={OS_URL[os()]} class="flex items-center gap-2" data-astro-prefetch="false">
      <span>DOWNLOAD FOR</span>
      <div class={`${OS_ICONS[os()]} w-4 h-4`} aria-hidden="true"></div>
    </a>
  );
};
