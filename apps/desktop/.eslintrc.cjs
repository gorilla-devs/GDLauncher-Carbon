module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true
  },
  overrides: [],
  rules: {
    "prettier/prettier": ["error", { endOfLine: "lf", trailingComma: "none" }],
    "no-unused-vars": [
      "error",
      { varsIgnorePattern: "^_", argsIgnorePattern: "^_" }
    ],
    "solid/reactivity": "off"
  },
  globals: {
    __APP_VERSION__: "readonly"
  },
  parser: "@typescript-eslint/parser",
  plugins: ["i18next", "solid", "prettier"],
  extends: [
    "eslint:recommended",
    "plugin:solid/typescript",
    "plugin:i18next/recommended",
    "prettier",
    "@unocss"
  ],
  ignorePatterns: ["dist/**/*", "node_modules/**/*"]
};
