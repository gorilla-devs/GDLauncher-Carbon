const config = require("@gd/config/tailwind.config");

module.exports = {
  ...config,
  content: ["./packages/mainWindow/**/*.{js,ts,jsx,tsx,html}"],
};
