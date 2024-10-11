import presetIcons from "@unocss/preset-icons";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unocss.theme.js";
import { presetScrollbarHide } from "unocss-preset-scrollbar-hide";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { supportedLanguages } from "@gd/i18n";
// import gdlIcons from "./unocss.icons.js";
import transformerDirectives from "@unocss/transformer-directives";

const safelist = Object.values(supportedLanguages).map(
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  (lang) => `i-emojione-v1:flag-for-${lang}`,
);

const unocssConfig = {
  content: {
    pipeline: {
      include: [
        /(apps\/desktop\/packages|packages\/ui)\/.*\.(ts|tsx|html|js|jsx)$/,
      ],
    },
  },
  presets: [
    presetAttributify(),
    presetWind(),
    // presetScrollbarHide(),
    presetIcons({
      // collections: {
      //   gdl: gdlIcons,
      // },
      // eslint-disable-next-line
      // @ts-ignore
      // hero: () =>
      //   import("@iconify-json/heroicons/icons.json").then((i) => i.default),
      ri: () => import("@iconify/json/json/ri.json").then((i) => i.default),
      "fa6-solid": () =>
        import("@iconify/json/json/fa6-solid.json").then((i) => i.default),
    }),
  ],
  rules: [],
  // rules: [["scrollbar-gutter", { "scrollbar-gutter": "stable" }]],
  safelist: [
    ...safelist,
    ...Object.keys(theme.colors).map((v) => `bg-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:bg-${v}-700`),
  ],
  theme,
  transformers: [transformerDirectives()],
} as unknown;

export { unocssConfig };
