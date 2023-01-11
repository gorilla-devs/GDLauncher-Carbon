import presetIcons from "@unocss/preset-icons";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unocss.theme.js";
// import gdlIcons from "./unocss.icons.js";

const unocssConfig = {
  include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
  presets: [
    presetAttributify(),
    presetWind(),
    presetIcons({
      // collections: {
      //   gdl: gdlIcons,
      // },
      // eslint-disable-next-line
      // @ts-ignore
      // hero: () =>
      //   import("@iconify-json/heroicons/icons.json").then((i) => i.default),
      ri: () => import("@iconify-json/ri/icons.json").then((i) => i.default),
    }),
  ],
  theme,
} as unknown;

export { unocssConfig };
