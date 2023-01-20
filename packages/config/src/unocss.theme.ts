type shade = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

type Theme = {
  // Colors
  primary: string;
  "primary-hover": string;
  shade: {
    [key in shade]: string;
  };
  blue: string;
  "blue-hover": string;
  red: string;
  "red-hover": string;
  yellow: string;
  "yellow-hover": string;
};

const theme = {
  colors: {
    primary: "var(--primary)",
    "primary-hover": "var(--primary-hover)",
    shade: {
      0: "var(--shade0)",
      1: "var(--shade1)",
      2: "var(--shade2)",
      3: "var(--shade3)",
      4: "var(--shade4)",
      5: "var(--shade5)",
      6: "var(--shade6)",
      7: "var(--shade7)",
      8: "var(--shade8)",
      9: "var(--shade9)",
    },
    blue: "var(--primary)",
    "blue-hover": "var(--primary-hover)",
    red: "var(--red)",
    "red-hover": "var(--red-hover)",
    yellow: "var(--yellow)",
    "yellow-hover": "var(--yellow-hover)",
    // TODO: Move satisfies to the entire theme when possible
  } satisfies Theme,
};

export { theme };
