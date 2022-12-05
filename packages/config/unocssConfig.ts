import path from "path";
import presetIcons from "@unocss/preset-icons";
import { readdirSync, readFileSync } from "fs";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";
import { theme } from "./unoCssTheme";

const gdlIcons = () => {
  let icons: { [key: string]: string } = {};
  try {
    const iconFiles = readdirSync(path.join(__dirname, "../", "ui", "icons"));
    for (const iconFile of iconFiles) {
      const file = readFileSync(
        path.join(__dirname, "../", "ui", "icons", iconFile)
      );

      icons[path.basename(iconFile, ".svg")] = file.toString();
    }
  } catch (error) {}

  return icons;
};

const unocssConfig: any = {
  include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
  presets: [
    presetAttributify({
      prefix: "uno:",
      prefixedOnly: true,
    }),
    presetWind(),
    presetIcons({
      collections: {
        gdl: gdlIcons(),
      },
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
      ([a, d]: [string, string]) => {
        let img = d.split("-")[0];
        return {
          background: `url('./assets/images/${img}')`,
          "background-size": "cover",
          "background-repeat": "no-repeat",
          "box-sizing": "border-box",
        };
      },
    ],
  ],
};

export { unocssConfig };
