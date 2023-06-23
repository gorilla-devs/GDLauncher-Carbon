import presetIcons from "@unocss/preset-icons";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unocss.theme.js";
import { presetScrollbarHide } from "unocss-preset-scrollbar-hide";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import supportedLanguages from "@gd/i18n/supportedLanguages.json";
// import gdlIcons from "./unocss.icons.js";

const safelist = Object.values(supportedLanguages).map(
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  (lang) => `i-emojione-v1:flag-for-${lang}`
);

const unocssConfig = {
  include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
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
      ri: () => import("@iconify/json/json/ri.json").then((i) => i.default),
      "devicon-plain": () =>
        import("@iconify/json/json/devicon-plain.json").then((i) => i.default),
    }),
  ],
  rules: [["scrollbar-gutter", { "scrollbar-gutter": "stable" }]],
  safelist,
  theme,
} as unknown;

export { unocssConfig };
