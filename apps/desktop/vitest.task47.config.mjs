import base from "./vitest.task31.config.mjs"
export default {
  ...base,
  test: {
    ...base.test,
    name: "task47",
    include: [
      "packages/mainWindow/src/**/*.task47.test.{ts,tsx}",
      // Task27 and these add-on button component tests share this environment.
      "packages/mainWindow/src/**/*.task27.test.{ts,tsx}",
      "packages/mainWindow/src/components/ModDownloadButton/dropdownFeedback.test.tsx",
      "packages/mainWindow/src/components/ModDownloadButton/primaryPresentation.test.tsx",
      "packages/mainWindow/src/components/ModDownloadButton/shaderPreflight.test.tsx",
      // Fix-round copy rendered through the shipped locale bundles.
      "packages/mainWindow/src/components/AddonFailures/panel.test.tsx",
      "packages/mainWindow/src/pages/Library/Instance/Tabs/Addons/components/DatapackStaging.test.tsx"
    ],
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@kobalte\//,
          /@solid-primitives\//,
          /@mbarzda\//
        ]
      }
    }
  }
}
