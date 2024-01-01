// Place any global data in this file.
// You can import this data from anywhere in your site by using the `import` keyword.

import Apple from "./assets/Apple";
import Linux from "./assets/Linux";
import Windows from "./assets/Windows";

export const SITE_TITLE = "Astro Blog";
export const SITE_DESCRIPTION = "Welcome to my website!";

export const DownloadItems: Array<{
  item: Element | string;
  onClick: () => void;
}> = [
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Apple /> MacOS
      </div>
    ) as Element,
    onClick: () => {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__mac__universal.dmg";
    },
  },
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Windows /> Windows
      </div>
    ) as Element,
    onClick: () => {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__win__x64.exe";
    },
  },
  {
    item: (
      <div class="flex items-center gap-2 p-1">
        <Linux /> Linux
      </div>
    ) as Element,
    onClick: () => {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__linux__x64.AppImage";
    },
  },
];
