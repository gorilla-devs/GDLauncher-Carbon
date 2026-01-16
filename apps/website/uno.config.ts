import { defineConfig } from "unocss"
import presetIcons from "@unocss/preset-icons"
import presetWind from "@unocss/preset-wind"
import { theme } from "@gd/config/unocss.theme"

export default defineConfig({
  content: {
    pipeline: {
      include: [
        /apps\/website\/.*\.(astro|ts|tsx|html|js|jsx)$/,
        /packages\/ui\/.*\.(ts|tsx)$/
      ]
    }
  },
  presets: [presetWind(), presetIcons()],
  theme,
  safelist: [
    // Brand colors for platform badges
    "bg-brands-curseforge",
    "bg-brands-modrinth",
    "bg-brands-discord",
    "bg-brands-bisecthosting",
    "text-brands-curseforge",
    "text-brands-modrinth",
    "text-brands-discord",
    "text-brands-bisecthosting",
    // Common utility classes
    ...Object.keys(theme.colors).flatMap((color) => [
      `bg-${color}-500`,
      `bg-${color}-600`,
      `bg-${color}-700`,
      `hover:bg-${color}-600`,
      `hover:bg-${color}-700`,
      `text-${color}-50`,
      `text-${color}-100`,
      `text-${color}-500`,
      `text-${color}-600`,
      `border-${color}-500`,
      `border-${color}-600`
    ])
  ],
  shortcuts: {
    // Website-specific shortcuts for backwards compatibility with existing Tailwind classes
    "text-mdgd": "text-[3.125rem]",
    "text-smgd": "text-[1.25rem]",
    "font-smgd": "text-[1.25rem]",
    "rounded-smgd": "rounded-[34px]",
    "rounded-xsgd": "rounded-[12px]",
    "rounded-xssgd": "rounded-[8px]",
    "p-mdgd": "p-[24px]",
    "shadow-mdgd": "shadow-[0px_0px_12px_0px_rgba(40,101,164,1)]",
    // Color mappings from old Tailwind config
    "bg-darkgd": "bg-darkSlate-800",
    "text-darkgd": "text-darkSlate-800",
    "text-graygd": "text-darkSlate-50",
    "text-whitegd": "text-lightSlate-50",
    "bg-bluegd-400": "bg-primary-300",
    "bg-bluegd-500": "bg-primary-500",
    "bg-bluegd-600": "bg-primary-700",
    "text-bluegd-400": "text-primary-300",
    "text-bluegd-500": "text-primary-500",
    "border-bluegd-400": "border-primary-300",
    "border-bluegd-500": "border-primary-500",
    // Spring easing utility
    "ease-spring": "ease-[cubic-bezier(0.34,1.56,0.64,1)]",
    // Common component patterns
    "btn-primary":
      "bg-primary-500 hover:bg-primary-600 text-lightSlate-50 rounded-lg px-4 py-2 transition-colors duration-200",
    "btn-secondary":
      "bg-darkSlate-600 hover:bg-darkSlate-500 text-lightSlate-50 rounded-lg px-4 py-2 transition-colors duration-200",
    "btn-ghost":
      "bg-transparent hover:bg-darkSlate-700 text-lightSlate-100 rounded-lg px-4 py-2 transition-colors duration-200",
    "card-base": "bg-darkSlate-700 rounded-xl p-4",
    "card-hover": "hover:bg-darkSlate-600 transition-colors duration-200"
  },
  rules: [
    // Custom animation utilities
    [
      "animate-fade-in",
      { animation: "fadeIn 0.2s ease-in-out forwards" }
    ],
    [
      "animate-fade-out",
      { animation: "fadeOut 0.2s ease-in-out forwards" }
    ],
    [
      "animate-slide-up",
      { animation: "slideUp 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards" }
    ],
    [
      "animate-scale-in",
      { animation: "scaleIn 0.2s cubic-bezier(0.34, 1.56, 0.64, 1) forwards" }
    ]
  ],
  preflights: [
    {
      getCSS: () => `
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes fadeOut {
          from { opacity: 1; }
          to { opacity: 0; }
        }
        @keyframes slideUp {
          from { opacity: 0; transform: translateY(10px); }
          to { opacity: 1; transform: translateY(0); }
        }
        @keyframes scaleIn {
          from { opacity: 0; transform: scale(0.95); }
          to { opacity: 1; transform: scale(1); }
        }
      `
    }
  ]
})
