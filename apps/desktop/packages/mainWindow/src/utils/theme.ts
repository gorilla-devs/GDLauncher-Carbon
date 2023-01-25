import { mainTheme, lightTheme, Theme, poisonGreen } from "@gd/ui";

import { createEffect, createSignal } from "solid-js";

const [theme, setTheme] = createSignal("default");

createEffect(() => {
  switch (theme()) {
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

export { theme, setTheme };
