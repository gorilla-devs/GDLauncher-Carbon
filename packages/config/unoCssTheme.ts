const theme = {
  colors: {
    accent: {
      main: "#2B6CB0",
      hover: "#3E86D0",
    },
    black: {
      black: "#1D2028",
      blackOpacity80: "rgba(29, 32, 40, 0.8)",
      semiblack: "#272B35",
      lightGray: "#8A8B8F",
      gray: "#404759",
    },
    status: {
      red: "#E54B4B",
      yellow: "#F7BC3D",
      green: "#29A335",
    },
  },
  extend: {
    keyframes: {
      bouncescale: {
        "0%": { transform: "scale(0)" },
        "100%": { transform: "scale(1)" },
      },
    },
  },
};
export { theme };
