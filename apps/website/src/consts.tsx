// Place any global data in this file.
// You can import this data from anywhere in your site by using the `import` keyword.

/** Canonical site origin. Single source of truth for absolute URLs in JSON-LD,
 *  canonical tags, and hand-built og:image fallbacks. */
export const SITE_URL = "https://gdlauncher.com";

export const APP_URLS = {
  cdn: "https://cdn.gdl.gg",
  socials: {
    discord: "https://discord.gdlauncher.com",
    github: "https://github.com/gorilla-devs/GDLauncher-Carbon",
    instagram: "https://www.instagram.com/gdlauncher",
    twitter: "https://twitter.com/gdlauncher",
  },
  newsletter: import.meta.env.ENDERIUM_API_BASE,
  olddownload: {
    win: "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-win-setup.exe",
    macOs:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-mac-setup.dmg",
    linux:
      "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-linux-setup.AppImage",
    releases: "https://github.com/gorilla-devs/GDLauncher/releases",
  },
};
if (!APP_URLS.newsletter) {
  // Fail loudly at build time rather than silently producing
  // `undefined/mailing` URLs that 404 in the browser.
  throw new Error(
    "ENDERIUM_API_BASE is not set. Add it to .env or the deploy environment.",
  );
}
export const ADD_USER_ENDPOINT = `${APP_URLS.newsletter}/mailing`;

export const SITE_TITLE =
  "GDLauncher: Free Modded Minecraft Launcher for CurseForge & Modrinth";
export const SITE_DESCRIPTION =
  "Free Minecraft launcher for mods and modpacks. One-click installs from CurseForge and Modrinth. Auto Java, auto updates, Cloud Instance Sharing. Windows, macOS, Linux.";

export const SITE_KEYWORDS =
  "minecraft launcher, modded minecraft launcher, minecraft mod launcher, minecraft modpack launcher, curseforge launcher, modrinth launcher, forge launcher, fabric launcher, neoforge launcher, best minecraft launcher, free minecraft launcher, gdlauncher";

export const TWITTER_HANDLE = "@gdlauncher";
