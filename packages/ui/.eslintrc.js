module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  overrides: [],
  rules: {
    "prettier/prettier": [
      "error",
      {
        endOfLine: "lf",
      },
    ],
    "no-unused-vars": "off",
    "@typescript-eslint/no-unused-vars": [
      "error",
      {
        varsIgnorePattern: "^_",
        argsIgnorePattern: "^_",
        caughtErrorsIgnorePattern: "^_",
      },
    ],
  },
  parser: "@typescript-eslint/parser",
  plugins: ["@typescript-eslint", "solid", "prettier"],
  extends: [
    "eslint:recommended",
    "plugin:solid/typescript",
    "prettier",
    "plugin:storybook/recommended",
  ],
  ignorePatterns: ["dist/**/*", "node_modules/**/*"],
};
