import {
  mainTheme,
  lightTheme,
  Theme as UITheme,
  poisonGreen,
  dracula
} from "@gd/ui";
import { createEffect } from "solid-js";
import { rspc } from "./rspcClient";

enum _Theme {
  _Default = "default",
  _Light = "light",
  _PoisonGreen = "poison-green",
  _Dracula = "dracula"
}

const initThemes = () => {
  let theme = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));
  const themeName = () => theme.data?.theme;
  createEffect(() => {
    if (!themeName()) {
      applyTheme(mainTheme);
      return;
    }
    switch (themeName()) {
      case _Theme._Light: {
        applyTheme(lightTheme);
        break;
      }
      case _Theme._PoisonGreen: {
        applyTheme(poisonGreen);
        break;
      }
      case _Theme._Dracula: {
        applyTheme(dracula);
        break;
      }
      default: {
        applyTheme(mainTheme);
        break;
      }
    }
  });
};

export function applyTheme(theme: UITheme) {
  // Inject theme
  for (const key in theme) {
    document.documentElement.style.setProperty(
      `--${key}`,
      theme[key as keyof UITheme]
    );
  }
}
export function getAvailableThemes(): string[] {
  return Object.values(_Theme);
}

export function getThemeColors(themeName: string): UITheme | undefined {
  switch (themeName) {
    case _Theme._Light: {
      return lightTheme;
    }
    case _Theme._PoisonGreen: {
      return poisonGreen;
    }
    case _Theme._Dracula: {
      return dracula;
    }
    default: {
      return mainTheme;
    }
  }
}

export default initThemes;
