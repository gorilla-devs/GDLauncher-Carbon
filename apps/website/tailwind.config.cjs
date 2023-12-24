/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        darkgd: "rgba(29, 32, 40, 1)",
        graygd: "rgba(147, 153, 170, 1)",
        whitegd: "rgba(255, 255, 255, 1)",
        bluegd: {
          400: "rgba(62, 134, 208, 1)",
          500: "rgba(40, 101, 164, 1)",
          600: "rgba(35, 62, 94, 1)",
        },
      },
      boxShadow: {
        mdgd: "0px 0px 12px 0px rgba(40, 101, 164, 1)",
      },
      borderRadius: {
        xssgd: "8px",
        xsgd: "12px",
        smgd: "34px",
      },
      fontSize: {
        smgd: "20px",
        mdgd: "50px",
      },
    },
  },
  plugins: [],
};
