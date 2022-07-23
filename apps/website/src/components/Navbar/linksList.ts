interface NavigationLink {
  title: [string, string];
  linkTo: string;
  isExternalLink?: boolean;
}

// Title is a [string, string] array because it needs to be translated reactively

export default [
  {
    title: ["home", "home"],
    linkTo: "/",
  },
  {
    title: ["features", "features"],
    linkTo: "/features",
  },
  {
    title: ["team", "team"],
    linkTo: "/team",
  },
  {
    title: ["contact", "contact"],
    linkTo: "/contact",
  },
  {
    title: ["github", "github"],
    linkTo: "https://github.com/gorilla-devs/GDLauncher",
    isExternalLink: true,
  },
  {
    title: ["discord", "discord"],
    linkTo: "https://discord.gdlauncher.com",
    isExternalLink: true,
  },
] as NavigationLink[];
