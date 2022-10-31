export const APP_URLS = {
  cdn: "https://edge.gdlauncher.com",
  socials: {
    discord: "https://discord.gdlauncher.com",
    github: "https://github.com/gorilla-devs/GDLauncher",
    instagram: "https://www.instagram.com/gdlauncher",
    twitter: "https://twitter.com/gdlauncher",
  },
  newsletter: "https://sendinblue-routing-test.cavallogianmarco.workers.dev/",
  kofi: "https://ko-fi.com/gdlauncher",
  download: {
    win: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-win-setup.exe",
    macOs:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-mac-setup.dmg",
    linux:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-linux-setup.AppImage",
    releases: "https://github.com/gorilla-devs/GDLauncher/releases",
  },
};

export const SUPPORTED_LANGUAGES = ["en"];
export const ADD_USER_ENDPOINT = `${APP_URLS.newsletter}/add`;
