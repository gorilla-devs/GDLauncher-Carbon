type shade = 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900

type Theme = {
  // Colors
  accent: string
  darkSlate: {
    [key in shade]: string
  }
  lightSlate: {
    [key in shade]: string
  }
  lightGray: {
    [key in shade]: string
  }
  darkGray: {
    [key in shade]: string
  }
  primary: {
    [key in shade]: string
  }
  red: {
    [key in shade]: string
  }
  yellow: {
    [key in shade]: string
  }
  green: {
    [key in shade]: string
  }
  brands: {
    [key: string]: string
  }
}

const theme = {
  colors: {
    accent: "rgb(var(--accent) / <alpha-value>)",
    darkSlate: {
      50: "rgb(var(--darkSlate-50) / <alpha-value>)",
      100: "rgb(var(--darkSlate-100) / <alpha-value>)",
      200: "rgb(var(--darkSlate-200) / <alpha-value>)",
      300: "rgb(var(--darkSlate-300) / <alpha-value>)",
      400: "rgb(var(--darkSlate-400) / <alpha-value>)",
      500: "rgb(var(--darkSlate-500) / <alpha-value>)",
      600: "rgb(var(--darkSlate-600) / <alpha-value>)",
      700: "rgb(var(--darkSlate-700) / <alpha-value>)",
      800: "rgb(var(--darkSlate-800) / <alpha-value>)",
      900: "rgb(var(--darkSlate-900) / <alpha-value>)"
    },
    lightSlate: {
      50: "rgb(var(--lightSlate-50) / <alpha-value>)",
      100: "rgb(var(--lightSlate-100) / <alpha-value>)",
      200: "rgb(var(--lightSlate-200) / <alpha-value>)",
      300: "rgb(var(--lightSlate-300) / <alpha-value>)",
      400: "rgb(var(--lightSlate-400) / <alpha-value>)",
      500: "rgb(var(--lightSlate-500) / <alpha-value>)",
      600: "rgb(var(--lightSlate-600) / <alpha-value>)",
      700: "rgb(var(--lightSlate-700) / <alpha-value>)",
      800: "rgb(var(--lightSlate-800) / <alpha-value>)",
      900: "rgb(var(--lightSlate-900) / <alpha-value>)"
    },
    darkGray: {
      50: "rgb(var(--darkGray-50) / <alpha-value>)",
      100: "rgb(var(--darkGray-100) / <alpha-value>)",
      200: "rgb(var(--darkGray-200) / <alpha-value>)",
      300: "rgb(var(--darkGray-300) / <alpha-value>)",
      400: "rgb(var(--darkGray-400) / <alpha-value>)",
      500: "rgb(var(--darkGray-500) / <alpha-value>)",
      600: "rgb(var(--darkGray-600) / <alpha-value>)",
      700: "rgb(var(--darkGray-700) / <alpha-value>)",
      800: "rgb(var(--darkGray-800) / <alpha-value>)",
      900: "rgb(var(--darkGray-900) / <alpha-value>)"
    },
    lightGray: {
      50: "rgb(var(--lightGray-50) / <alpha-value>)",
      100: "rgb(var(--lightGray-100) / <alpha-value>)",
      200: "rgb(var(--lightGray-200) / <alpha-value>)",
      300: "rgb(var(--lightGray-300) / <alpha-value>)",
      400: "rgb(var(--lightGray-400) / <alpha-value>)",
      500: "rgb(var(--lightGray-500) / <alpha-value>)",
      600: "rgb(var(--lightGray-600) / <alpha-value>)",
      700: "rgb(var(--lightGray-700) / <alpha-value>)",
      800: "rgb(var(--lightGray-800) / <alpha-value>)",
      900: "rgb(var(--lightGray-900) / <alpha-value>)"
    },
    primary: {
      50: "rgb(var(--primary-50) / <alpha-value>)",
      100: "rgb(var(--primary-100) / <alpha-value>)",
      200: "rgb(var(--primary-200) / <alpha-value>)",
      300: "rgb(var(--primary-300) / <alpha-value>)",
      400: "rgb(var(--primary-400) / <alpha-value>)",
      500: "rgb(var(--primary-500) / <alpha-value>)",
      600: "rgb(var(--primary-600) / <alpha-value>)",
      700: "rgb(var(--primary-700) / <alpha-value>)",
      800: "rgb(var(--primary-800) / <alpha-value>)",
      900: "rgb(var(--primary-900) / <alpha-value>)"
    },
    red: {
      50: "rgb(var(--red-50) / <alpha-value>)",
      100: "rgb(var(--red-100) / <alpha-value>)",
      200: "rgb(var(--red-200) / <alpha-value>)",
      300: "rgb(var(--red-300) / <alpha-value>)",
      400: "rgb(var(--red-400) / <alpha-value>)",
      500: "rgb(var(--red-500) / <alpha-value>)",
      600: "rgb(var(--red-600) / <alpha-value>)",
      700: "rgb(var(--red-700) / <alpha-value>)",
      800: "rgb(var(--red-800) / <alpha-value>)",
      900: "rgb(var(--red-900) / <alpha-value>)"
    },
    yellow: {
      50: "rgb(var(--yellow-50) / <alpha-value>)",
      100: "rgb(var(--yellow-100) / <alpha-value>)",
      200: "rgb(var(--yellow-200) / <alpha-value>)",
      300: "rgb(var(--yellow-300) / <alpha-value>)",
      400: "rgb(var(--yellow-400) / <alpha-value>)",
      500: "rgb(var(--yellow-500) / <alpha-value>)",
      600: "rgb(var(--yellow-600) / <alpha-value>)",
      700: "rgb(var(--yellow-700) / <alpha-value>)",
      800: "rgb(var(--yellow-800) / <alpha-value>)",
      900: "rgb(var(--yellow-900) / <alpha-value>)"
    },
    green: {
      50: "rgb(var(--green-50) / <alpha-value>)",
      100: "rgb(var(--green-100) / <alpha-value>)",
      200: "rgb(var(--green-200) / <alpha-value>)",
      300: "rgb(var(--green-300) / <alpha-value>)",
      400: "rgb(var(--green-400) / <alpha-value>)",
      500: "rgb(var(--green-500) / <alpha-value>)",
      600: "rgb(var(--green-600) / <alpha-value>)",
      700: "rgb(var(--green-700) / <alpha-value>)",
      800: "rgb(var(--green-800) / <alpha-value>)",
      900: "rgb(var(--green-900) / <alpha-value>)"
    },
    brands: {
      curseforge: "rgb(var(--brands-curseforge) / <alpha-value>)",
      modrinth: "rgb(var(--brands-modrinth) / <alpha-value>)",
      discord: "rgb(var(--brands-discord) / <alpha-value>)",
      bisecthosting: "rgb(var(--brands-bisecthosting) / <alpha-value>)"
    }
    // TODO: Move satisfies to the entire theme when possible
  } satisfies Theme,
  animation: {
    keyframes: {
      loadingbar:
        "{ 0% { transform: translateX(0) scaleX(0); } 40% { transform: translateX(0) scaleX(0.4); } 100% { transform: translateX(100%) scaleX(0.5); } }",
      enterWithOpacityChange: "{ 0% { opacity: 0 } 100% { opacity: 1 } }",
      scaleBounce:
        "{ 0% { transform: scale(1); } 50% { transform: scale(0); } 70% { transform: scale(1); } 85% { transform: scale(1.2); } 100% { transform: scale(1); } }",
      wiggle:
        "{ 0% { transform: rotate(0deg); } 25% { transform: rotate(10deg); } 50% { transform: rotate(-10deg); } 75% { transform: rotate(5deg); } 100% { transform: rotate(0deg); } }",
      liveCirclePulse:
        " { 0% { box-shadow: 0 0 0 0 rgba(255,0,0, 0.4); } 70% { box-shadow: 0 0 0 15px rgba(255,0,0, 0); } 100% { box-shadow: 0 0 0 0 rgba(255,0,0, 0); } }",
      menuEnter:
        "{ 0% { opacity: 0; transform: scale(0); } 100% { opacity: 1; transform: scale(1); } }",
      menuLeave:
        "{ 0% { opacity: 1; transform: scale(1); } 100% { opacity: 0; transform: scale(0); } }"
    },

    durations: {
      loadingbar: "1s",
      enterWithOpacityChange: "200ms",
      scaleBounce: "600ms",
      wiggle: "300ms",
      liveCirclePulse: "1s",
      menuEnter: "180ms",
      menuLeave: "150ms"
    },
    timingFns: {
      loadingbar: "linear",
      enterWithOpacityChange: "ease-in-out",
      scaleBounce: "cubic-bezier(0.68, -0.55, 0.265, 1.55)",
      wiggles: "ease-in-out",
      liveCirclePulse: "ease-in-out",
      menuEnter: "cubic-bezier(0.4, 0.0, 0.2, 1)",
      menuLeave: "cubic-bezier(0.4, 0.0, 0.2, 1)"
    },
    counts: {
      loadingbar: "infinite",
      enterWithOpacityChange: "forwards",
      liveCirclePulse: "infinite",
      menuEnter: "forwards",
      menuLeave: "forwards"
    }
  },
  screens: {
    xs: { max: "800px" }, // For the condition width <= 800
    sm: { max: "999px" }, // For the condition width < 1000
    md: { max: "1499px" } // For the condition width < 1500
    // Any width >= 1500 will be considered the default or 'lg' scenario
  }
}

export { theme }
