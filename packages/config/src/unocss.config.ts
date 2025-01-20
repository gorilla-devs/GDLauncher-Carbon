import presetIcons from "@unocss/preset-icons"
import presetWind from "@unocss/preset-wind"
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
  presets: [presetWind(), presetIcons()],
  rules: [],
  safelist: [
    ...safelist,
    ...Object.keys(theme.colors).map((v) => `bg-${v}-500`),
    ...Object.keys(theme.colors).map((v) => `hover:bg-${v}-700`)
  ],
  theme,
  transformers: []
} as unknown

export { unocssConfig }
