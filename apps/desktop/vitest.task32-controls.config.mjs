import base from "./vitest.task32.config.mjs"
export default {
  ...base,
  test: {
    ...base.test,
    include: [
      "packages/mainWindow/src/managers/ModalsManager/modals/DuplicatedModsResolution/duplicates.task31.test.tsx",
      "packages/mainWindow/src/managers/ModalsManager/modals/updater.task31.test.tsx",
      "packages/mainWindow/src/managers/ModalsManager/javaOwnership.task29.test.tsx",
      "packages/mainWindow/src/managers/ModalsManager/resolvePreventClose.test.ts",
      "packages/mainWindow/src/managers/ModalsManager/modals/JavaProfileCreationModal.test.tsx",
      "packages/mainWindow/src/pages/Library/Instance/Tabs/Addons/hooks/useAddonMutations.test.ts",
      "packages/main/utils/__tests__/runtimeRelocation.test.ts",
      "packages/mainWindow/src/pages/Search/FilterSidebar/controls.task28.test.tsx"
    ]
  }
}
