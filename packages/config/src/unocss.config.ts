import presetIcons from "@unocss/preset-icons";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unocss.theme.js";
import { presetScrollbarHide } from "unocss-preset-scrollbar-hide";
// import gdlIcons from "./unocss.icons.js";

const unocssConfig = {
  content: {
    pipeline: {
      include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
    },
  },
  presets: [
    presetAttributify(),
    presetWind(),
    presetScrollbarHide(),
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
  rules: [],
  theme,
} as unknown;

export { unocssConfig };
