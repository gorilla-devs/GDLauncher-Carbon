/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        darkgd: "rgba(29, 32, 40, 1)",
        bluegd: "rgba(40, 101, 164, 1)",
        graygd: "rgba(147, 153, 170, 1)",
        whitegd: "rgba(255, 255, 255, 1)",
      },
      boxShadow: {
        md: "0px 0px 12px 0px rgba(40, 101, 164, 1)",
      },
    },
  },
  plugins: [],
};
