type Theme = {
  // Colors
  primary: string;
  "primary-hover": string;
  shade0: string;
  shade1: string;
  shade2: string;
  shade3: string;
  shade4: string;
  shade5: string;
  shade6: string;
  shade7: string;
  shade8: string;
  shade9: string;
  red: string;
  "red-hover": string;
  yellow: string;
  "yellow-hover": string;
};

export type TGenericTheme = {
  [key in keyof Theme]: `var(--${keyof Theme})`;
};

const theme = {
  colors: {
    primary: "var(--primary)",
    "primary-hover": "var(--primary-hover)",
    shade0: "var(--shade0)",
    shade1: "var(--shade1)",
    shade2: "var(--shade2)",
    shade3: "var(--shade3)",
    shade4: "var(--shade4)",
    shade5: "var(--shade5)",
    shade6: "var(--shade6)",
    shade7: "var(--shade7)",
    shade8: "var(--shade8)",
    shade9: "var(--shade9)",
    red: "var(--red)",
    "red-hover": "var(--red-hover)",
    yellow: "var(--yellow)",
    "yellow-hover": "var(--yellow-hover)",
    // TODO: Move satisfies to the entire theme when possible
  } as TGenericTheme,
  // TODO: check how to implement animations in unocss
  keyframes: {
    bouncescale: {
      "0%": { transform: "scale(0)" },
      "100%": { transform: "scale(1)" },
    },
  },
  animation: {
    "bounce-scale": "bouncescale 1s ease-in-out infinite",
  },
};

export { theme };
