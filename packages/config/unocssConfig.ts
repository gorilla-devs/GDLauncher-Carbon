import path from "path";
import presetIcons from "@unocss/preset-icons";
import { readdirSync, readFileSync } from "fs";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";

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

const config: any = {
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
  theme: {
    colors: {
      accent: {
        main: "#2B6CB0",
        hover: "#3E86D0",
      },
      black: {
        black: "#1D2028",
        blackOpacity80: "rgba(29, 32, 40, 0.8)",
        semiblack: "#272B35",
        lightGray: "#8A8B8F",
        gray: "#404759",
      },
      status: {
        red: "#E54B4B",
        yellow: "#F7BC3D",
        green: "#29A335",
      },
    },
  },
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

export default config;
