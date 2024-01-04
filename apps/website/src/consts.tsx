// Place any global data in this file.
// You can import this data from anywhere in your site by using the `import` keyword.

import Apple from "./assets/Apple";
import Linux from "./assets/Linux";
import Windows from "./assets/Windows";

export const APP_URLS = {
  socials: {
    discord: "https://discord.gdlauncher.com",
    github: "https://github.com/gorilla-devs/GDLauncher",
    instagram: "https://www.instagram.com/gdlauncher",
    twitter: "https://twitter.com/gdlauncher",
  },
  newsletter: "https://api.gdlauncher.com/v1",
  kofi: "https://ko-fi.com/gdlauncher",
  olddownload: {
    win: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-win-setup.exe",
    macOs:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-mac-setup.dmg",
    linux:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-linux-setup.AppImage",
    releases: "https://github.com/gorilla-devs/GDLauncher/releases",
  },
};
export const ADD_USER_ENDPOINT = `${APP_URLS.newsletter}/mailing`;

export const SITE_TITLE = "GdLauncher";
export const SITE_DESCRIPTION = "Welcome to gdlauncher!";

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
