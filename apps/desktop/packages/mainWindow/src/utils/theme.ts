import { mainTheme, lightTheme, Theme } from "@gd/ui";

import { createEffect, createSignal } from "solid-js";

const [theme, setTheme] = createSignal(0);

createEffect(() => {
  switch (theme()) {
    case 0: {
      applyTheme(mainTheme);
      break;
    }
    case 1: {
      applyTheme(lightTheme);
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
