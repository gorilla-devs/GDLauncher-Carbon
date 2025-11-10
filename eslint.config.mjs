import js from "@eslint/js"
import ts from "typescript-eslint"
import tailwind from "eslint-plugin-tailwindcss"
import solid from "eslint-plugin-solid/configs/recommended"
import prettier from "eslint-plugin-prettier/recommended"
import gitignore from "eslint-config-flat-gitignore"
import globals from "globals"
import turboPlugin from "eslint-plugin-turbo"
// import perfectionist from "eslint-plugin-perfectionist";
import i18nLint from "eslint-plugin-i18next"

/** @type {import("eslint").Linter.Config} */
export default [
  // Respect the .gitignore
  gitignore(),

  // JavaScript
  js.configs.recommended,

  // TypeScript
  ...ts.configs.recommendedTypeChecked,
  ...ts.configs.stylisticTypeChecked,

  {
    plugins: {
      turbo: turboPlugin
    },
    rules: {
      "turbo/no-undeclared-env-vars": "warn"
    }
  },
  {
    languageOptions: {
      parserOptions: {
        project: true,
        tsconfigRootDir: import.meta.dirname
      }
    }
  },

  // JavaScript...again
  {
    files: ["**/*.cjs", "**/*.js", "**/*.jsx", "**/*.mjs"],
    ...ts.configs.disableTypeChecked
  },

  // Pedanticism
  // perfectionist.configs["recommended-natural"],

  // JSX/Solid/TailwindCSS
  solid,
  ...tailwind.configs["flat/recommended"],

  i18nLint.configs["flat/recommended"],

  // Prettier...because prettier
  prettier,

  // Rule overrides
  {
    rules: {
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
    }
  },

  // By default use node globals
  {
    languageOptions: { globals: globals.node }
  },

  // Define browser envs
  {
    files: [
      "apps/desktop/packages/mainWindow/src/**/*",
      "apps/website/src/**/*",
      "packages/ui/src/**/*",
      "packages/i18n/src/**/*"
    ],
    languageOptions: { globals: globals.browser }
  },

  // App global
  {
    files: ["apps/desktop/packages/**/*"],

    languageOptions: {
      globals: {
        __APP_VERSION__: "readonly"
      }
    }
  },

  // temporarily disabled rules
  {
    rules: {
      "@typescript-eslint/prefer-nullish-coalescing": "off",
      "tailwindcss/no-custom-classname": "off",
      "tailwindcss/enforces-shorthand": "off",
      "@typescript-eslint/no-unsafe-return": "off",
      "@typescript-eslint/no-floating-promises": "off",
      "@typescript-eslint/no-unsafe-argument": "off",
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/require-await": "off",
      "@typescript-eslint/no-unsafe-member-access": "off",
      "@typescript-eslint/no-unsafe-assignment": "off",
      "@typescript-eslint/no-unsafe-call": "off",
      "@typescript-eslint/no-misused-promises": "off",
      "tailwindcss/no-contradicting-classname": "off",
      "@typescript-eslint/no-non-null-asserted-optional-chain": "off",
      "@typescript-eslint/no-empty-object-type": "off",
      "@typescript-eslint/no-unsafe-enum-comparison": "off",
      "@typescript-eslint/unbound-method": "off",
      "@typescript-eslint/no-empty-function": "off",
      "@typescript-eslint/restrict-plus-operands": "off",
      "@typescript-eslint/prefer-for-of": "off",
      "@typescript-eslint/ban-ts-comment": "off",
      "@typescript-eslint/no-require-imports": "off",
      "i18next/no-literal-string": "off"
    }
  },

  // Enable i18n literal string checking only for mainWindow
  {
    files: ["apps/desktop/packages/mainWindow/**/*"],
    rules: {
      "i18next/no-literal-string": "error"
    }
  },

  // temporarily disabled files
  {
    ignores: ["packages/ui/.storybook/**/*"]
  }
]
