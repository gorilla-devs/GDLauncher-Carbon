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
  _Main = "main",
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
    applyThemeByName(themeName());
  });
};

export function applyThemeByName(themeName: string | undefined) {
  if (!themeName) {
    applyTheme(mainTheme);
    return;
  }

  switch (themeName) {
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
}

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

export function getThemeColor(themeName: string, color: keyof UITheme): string {
  switch (themeName) {
    case _Theme._Light: {
      return `rgb(${lightTheme[color]})`;
    }
    case _Theme._PoisonGreen: {
      return `rgb(${poisonGreen[color]})`;
    }
    case _Theme._Dracula: {
      return `rgb(${dracula[color]})`;
    }
    default: {
      return `rgb(${mainTheme[color]})`;
    }
  }
}

export default initThemes;
