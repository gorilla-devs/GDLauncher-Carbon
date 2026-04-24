import presetIcons from "@unocss/preset-icons"
import presetWind3 from "@unocss/preset-wind3"
import { theme } from "./unocss.theme.js"
import { supportedLanguages } from "@gd/i18n"

const safelist = Object.values(supportedLanguages).map(
  (lang) => `i-emojione-v1:flag-for-${lang}`
)

const unocssConfig = {
  content: {
    pipeline: {
      include: [
        /(apps\/desktop\/packages|packages\/ui)\/.*\.(ts|tsx|html|js|jsx)$/
      ]
    }
  },
  presets: [presetWind3(), presetIcons()],
  rules: [],
  safelist: [
    ...safelist,
    ...Object.keys(theme.colors).map((v) => `bg-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:bg-${v}-700`),
    ...Object.keys(theme.colors).map((v) => `border-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:border-${v}-700`),
    ...Object.keys(theme.colors).map((v) => `text-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:text-${v}-700`),
    ...Object.keys(theme.colors).map((v) => `fill-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:fill-${v}-700`),
    ...Object.keys(theme.colors).map((v) => `stroke-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:stroke-${v}-700`)
  ],
  theme,
  transformers: []
} as unknown

export { unocssConfig }
