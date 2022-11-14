import path from "path";
import Unocss from "unocss/vite";
import presetIcons from "@unocss/preset-icons";
import { readdirSync, readFileSync } from "fs";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";

const gdlIcons = () => {
  let icons = {};

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

const config = {
  unoCss: Unocss({
    include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
    presets: [
      presetAttributify({
        prefix: "uno:",
        prefixedOnly: true,
      }),
      presetWind(),
      presetIcons({
        collections: {
          hero: () =>
            import("@iconify-json/heroicons/icons.json").then((i) => i.default),
          gdl: gdlIcons(),
        },
      }),
    ],
    theme: {
      colors: {
        accent: {
          main: "#2B6CB0",
        },
        black: {
          black: "#1D2028",
          blackOpacity80: "rgba(29, 32, 40, 0.8)",
        },
        semiblack: "#272B35"
      },
    },
    rules: [
      [
        /^bg-image-(.*)$/,
        ([a, d]) => {
          let img = d.split("-")[0];
          let extension = a.split(".")[1];
          const isSvg = extension === "svg";
          return {
            background: `url('./${
              process.env.NODE_ENV === "development" ? "assets/" : ""
            }images/${isSvg ? img : `${img}.png`}')`,
            "background-size": "cover",
            "background-repeat": "no-repeat",
            "box-sizing": "border-box",
          };
        },
      ],
    ],
  }),
};

export default config;
