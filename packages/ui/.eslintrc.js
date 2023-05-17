module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true
  },
  overrides: [],
  rules: {
    "prettier/prettier": ["error", {
      endOfLine: "lf"
    }],
    "no-unused-vars": ["error", {
      varsIgnorePattern: "^_",
      argsIgnorePattern: "^_"
    }]
  },
  parser: "@typescript-eslint/parser",
  plugins: ["solid", "prettier", "@unocss"],
  extends: ["eslint:recommended", "plugin:solid/typescript", "prettier", "plugin:storybook/recommended"],
  ignorePatterns: ["dist/**/*", "node_modules/**/*"]
};