import { mainTheme, lightTheme, Theme as UITheme, poisonGreen } from "@gd/ui";
import { createEffect } from "solid-js";
import { rspc } from "./rspcClient";

enum _Theme {
  _Default = "default",
  _Light = "light",
  _PoisonGreen = "poison-green",
}

const initThemes = () => {
  let theme = rspc.createQuery(() => ["settings.getSettings"]);
  const themeName = () => theme.data?.theme;
  createEffect(() => {
    if (!themeName()) {
      applyTheme(mainTheme);
      return;
    }
    switch (themeName()) {
      case _Theme._Default: {
        applyTheme(mainTheme);
        break;
      }
      case _Theme._Light: {
        applyTheme(lightTheme);
        break;
      }
      case _Theme._PoisonGreen: {
        applyTheme(poisonGreen);
        break;
      }
    }
  });

  function applyTheme(theme: UITheme) {
    // Inject theme
    for (const key in theme) {
      document.documentElement.style.setProperty(
        `--${key}`,
        theme[key as keyof UITheme]
      );
    }
  }
};

export default initThemes;
