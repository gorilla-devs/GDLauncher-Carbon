import presetIcons from "@unocss/preset-icons";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unocss.theme.js";
import gdlIcons from "./unocss.icons.js";

const unocssConfig = {
  include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
  presets: [
    presetAttributify({
      prefix: "u:",
      prefixedOnly: true,
    }),
    presetWind(),
    presetIcons({
      collections: {
        gdl: gdlIcons,
      },
      // eslint-disable-next-line
      // @ts-ignore
      hero: () =>
        import("@iconify-json/heroicons/icons.json").then((i) => i.default),
      ri: () => import("@iconify-json/ri/icons.json").then((i) => i.default),
    }),
  ],
  theme,
  rules: [
    [
      /^bg-image-(.*)$/,
      ([, d]: [string, string]) => {
        const img = d.split("-")[0];
        return {
          background: `url('./assets/images/${img}')`,
          "background-size": "cover",
          "background-repeat": "no-repeat",
          "box-sizing": "border-box",
        };
      },
    ],
  ],
} as unknown;

export { unocssConfig };
