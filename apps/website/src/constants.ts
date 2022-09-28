export const APP_URLS = {
  cdn: "https://gdlauncher.b-cdn.net/assets",
  waitlist: "https://fastapi-production-354a.up.railway.app",
  kofi: "https://ko-fi.com/gdlauncher",
  download: {
    win: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-win-setup.exe",
    macOs: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-mac-setup.dmg",
    linux: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-linux-setup.AppImage"
  }
};

export const SUPPORTED_LANGUAGES = ["en", "it"];
export const ADD_USER_ENDPOINT = `${APP_URLS.waitlist}/users/add`;
