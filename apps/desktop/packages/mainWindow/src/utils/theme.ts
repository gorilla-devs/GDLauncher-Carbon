import { mainTheme, lightTheme, Theme, poisonGreen } from "@gd/ui";
import { createEffect } from "solid-js";
import { rspc } from "./rspcClient";

const initThemes = () => {
  let theme = rspc.createQuery(() => ["app.getTheme", null]);

  createEffect(() => {
    if (!theme.data) {
      applyTheme(mainTheme);
      return;
    }
    switch (theme.data) {
      case "default": {
        applyTheme(mainTheme);
        break;
      }
      case "light": {
        applyTheme(lightTheme);
        break;
      }
      case "poison-green": {
        applyTheme(poisonGreen);
        break;
      }
    }
  });

  function applyTheme(theme: Theme) {
    // Inject theme
    for (const key in theme) {
      document.documentElement.style.setProperty(
        `--${key}`,
        theme[key as keyof Theme]
      );
    }
  }
};

export default initThemes;
