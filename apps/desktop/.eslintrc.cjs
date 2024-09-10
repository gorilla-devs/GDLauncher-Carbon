module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true
  },
  overrides: [],
  rules: {
    "prettier/prettier": ["error", { endOfLine: "lf", trailingComma: "none" }],
    "solid/reactivity": "off",
    "no-unused-vars": "off",
    "@typescript-eslint/no-unused-vars": [
      "error",
      {
        varsIgnorePattern: "^_",
        argsIgnorePattern: "^_",
        caughtErrorsIgnorePattern: "^_"
      }
    ]
  },
  globals: {
    __APP_VERSION__: "readonly"
  },
  parser: "@typescript-eslint/parser",
  plugins: ["@typescript-eslint", "i18next", "solid", "prettier"],
  extends: [
    "eslint:recommended",
    "plugin:solid/typescript",
    "plugin:i18next/recommended",
    "prettier"
  ],
  ignorePatterns: ["dist/**/*", "node_modules/**/*"]
};
