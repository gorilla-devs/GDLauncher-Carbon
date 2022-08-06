/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./packages/renderer/index.html",
    "./packages/renderer/src/**/*.{js,ts,jsx,tsx}",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
};
