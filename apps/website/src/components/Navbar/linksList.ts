import { UIDictionaryKeys } from "@/i18n/utils";

interface NavigationLink {
  title: [UIDictionaryKeys];
  linkTo: string;
  isExternalLink?: boolean;
}
export default [
  {
    title: ["navbar.home"],
    linkTo: "/",
  },
  // {
  //   title: ["navbar.features"],
  //   linkTo: "/features",
  // },
  {
    title: ["navbar.team"],
    linkTo: "/team",
  },
  {
    title: ["navbar.blog"],
    linkTo: "/blog",
  },
  {
    title: ["navbar.github"],
    linkTo: "https://github.com/gorilla-devs/GDLauncher",
    isExternalLink: true,
  },
  {
    title: ["navbar.discord"],
    linkTo: "https://discord.gdlauncher.com",
    isExternalLink: true,
  },
] as NavigationLink[];
