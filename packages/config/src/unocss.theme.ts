type shade = 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900;

type Theme = {
  // Colors
  accent: string;
  primary: string;
  "primary-hover": string;
  darkSlate: {
    [key in shade]: string;
  };
  lightSlate: {
    [key in shade]: string;
  };
  lightGray: {
    [key in shade]: string;
  };
  darkGray: {
    [key in shade]: string;
  };
  blue: {
    [key in shade]: string;
  };
  "blue-hover": string;
  red: string;
  "red-hover": string;
  yellow: string;
  "yellow-hover": string;
};

const theme = {
  colors: {
    accent: "var(--accent)",
    primary: "var(--primary)",
    "primary-hover": "var(--primary-hover)",
    darkSlate: {
      50: "var(--darkSlate-50)",
      100: "var(--darkSlate-100)",
      200: "var(--darkSlate-200)",
      300: "var(--darkSlate-300)",
      400: "var(--darkSlate-400)",
      500: "var(--darkSlate-500)",
      600: "var(--darkSlate-600)",
      700: "var(--darkSlate-700)",
      800: "var(--darkSlate-800)",
      900: "var(--darkSlate-900)",
    },
    lightSlate: {
      50: "var(--lightSlate-50)",
      100: "var(--lightSlate-100)",
      200: "var(--lightSlate-200)",
      300: "var(--lightSlate-300)",
      400: "var(--lightSlate-400)",
      500: "var(--lightSlate-500)",
      600: "var(--lightSlate-600)",
      700: "var(--lightSlate-700)",
      800: "var(--lightSlate-800)",
      900: "var(--lightSlate-900)",
    },
    darkGray: {
      50: "var(--darkGray-50)",
      100: "var(--darkGray-100)",
      200: "var(--darkGray-200)",
      300: "var(--darkGray-300)",
      400: "var(--darkGray-400)",
      500: "var(--darkGray-500)",
      600: "var(--darkGray-600)",
      700: "var(--darkGray-700)",
      800: "var(--darkGray-800)",
      900: "var(--darkGray-900)",
    },
    lightGray: {
      50: "var(--lightGray-50)",
      100: "var(--lightGray-100)",
      200: "var(--lightGray-200)",
      300: "var(--lightGray-300)",
      400: "var(--lightGray-400)",
      500: "var(--lightGray-500)",
      600: "var(--lightGray-600)",
      700: "var(--lightGray-700)",
      800: "var(--lightGray-800)",
      900: "var(--lightGray-900)",
    },
    blue: {
      50: "var(--blue-50)",
      100: "var(--blue-100)",
      200: "var(--blue-200)",
      300: "var(--blue-300)",
      400: "var(--blue-400)",
      500: "var(--blue-500)",
      600: "var(--blue-600)",
      700: "var(--blue-700)",
      800: "var(--blue-800)",
      900: "var(--blue-900)",
    },
    "blue-hover": "var(--primary-hover)",
    red: "var(--red)",
    "red-hover": "var(--red-hover)",
    yellow: "var(--yellow)",
    "yellow-hover": "var(--yellow-hover)",
    // TODO: Move satisfies to the entire theme when possible
  } satisfies Theme,
  animation: {
    keyframes: {
      loadingbar:
        "{ 0% { transform: translateX(0) scaleX(0); } 40% { transform: translateX(0) scaleX(0.4); } 100% { transform: translateX(100%) scaleX(0.5); } }",
    },
    duration: { loadingbar: "1s" },
    timingFns: { loadingbar: "linear" },
    counts: { loadingbar: "infinite" },
  },
};

export { theme };
