module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  overrides: [],
  rules: {
    "prettier/prettier": ["error", { endOfLine: "lf" }],
  },
  parser: "@typescript-eslint/parser",
  plugins: ["i18next", "solid", "prettier"],
  extends: [
    "eslint:recommended",
    "plugin:solid/typescript",
    "plugin:i18next/recommended",
    "prettier",
  ],
  ignorePatterns: ["dist/**/*", "node_modules/**/*"],
};
