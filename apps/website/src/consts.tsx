// Place any global data in this file.
// You can import this data from anywhere in your site by using the `import` keyword.

import Apple from "./assets/Apple";
import Linux from "./assets/Linux";
import Windows from "./assets/Windows";

export const APP_URLS = {
  cdn: "https://edge.gdlauncher.com",
  socials: {
    discord: "https://discord.gdlauncher.com",
    github: "https://github.com/gorilla-devs/GDLauncher-Carbon",
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
}> = [
  {
    item: (
      <a class="flex items-center gap-2 p-1">
        <Apple /> MacOS
      </a>
    ) as Element,
  },
  {
    item: (
      <a class="flex items-center gap-2 p-1">
        <Windows /> Windows
      </a>
    ) as Element,
  },
  {
    item: (
      <a class="flex items-center gap-2 p-1">
        <Linux /> Linux
      </a>
    ) as Element,
  },
];
