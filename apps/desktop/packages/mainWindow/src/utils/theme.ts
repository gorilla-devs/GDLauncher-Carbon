import { mainTheme, Theme as UITheme, pixelato } from "@gd/ui";
import { createEffect } from "solid-js";
import { rspc } from "./rspcClient";

enum _Theme {
  _Main = "main",
  _Pixelato = "pixelato"
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
    case _Theme._Pixelato: {
      applyTheme(pixelato);
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
    if (key === "additional-styles") {
      if (document.getElementById(key)) {
        document.getElementById(key)?.remove();
      }

      const style = document.createElement("style");
      style.setAttribute("id", key);
      style.innerHTML = theme[key as keyof UITheme];
      document.head.appendChild(style);
      continue;
    }

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
    case _Theme._Pixelato: {
      return `rgb(${pixelato[color]})`;
    }
    default: {
      return `rgb(${mainTheme[color]})`;
    }
  }
}

export default initThemes;
