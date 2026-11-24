import { readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { describe, it, expect } from "vitest"
import {
  GDL_HOOKS,
  GDL_IDENTITIES,
  GDL_IDENTITY_TOKENS,
  MODAL_IDS,
  identityHook,
  kebabId,
  settingsPageId
} from "../hooks"

// Identity tokens say which one an element is: `<base>.<id>`. The lists are
// checked against the code that emits them.

const root = (path: string) =>
  fileURLToPath(new URL(`../../../../../../../${path}`, import.meta.url))
const read = (path: string) => readFileSync(root(path), "utf8")
const app = "apps/desktop/packages/mainWindow/src/"

describe("identity tokens", () => {
  it("are kebab-case ids", () => {
    expect(kebabId("instanceCreation")).toBe("instance-creation")
    expect(kebabId("modpack_version_update")).toBe("modpack-version-update")
    expect(kebabId("ConfirmChangeRuntimePath")).toBe(
      "confirm-change-runtime-path"
    )
    expect(kebabId("confirmGDLAccountDeletion")).toBe(
      "confirm-gdl-account-deletion"
    )
    expect(kebabId("runtime-path")).toBe("runtime-path")
    expect(identityHook("modal", "themeInstall")).toBe("modal.theme-install")
    for (const token of GDL_IDENTITY_TOKENS) {
      expect(token, token).toMatch(/^[a-z][a-z.-]*\.[a-z0-9]+(-[a-z0-9]+)*$/)
    }
  })

  it("name a base hook, and are never hooks themselves", () => {
    for (const base of Object.keys(GDL_IDENTITIES)) {
      expect(GDL_HOOKS, base).toContain(base)
    }
    for (const token of GDL_IDENTITY_TOKENS) {
      expect(GDL_HOOKS, token).not.toContain(token)
    }
    expect(new Set(GDL_IDENTITY_TOKENS).size).toBe(GDL_IDENTITY_TOKENS.length)
  })

  it("give every dialog in the dialog manager an id, and no other", () => {
    const source = read(`${app}managers/ModalsManager/index.tsx`)
    const registry = /const getDefaultModals = [\s\S]*?\n\}\)\n/.exec(
      source
    )![0]
    const names = [...registry.matchAll(/^ {2}([A-Za-z_]+): \{$/gm)].map(
      (m) => m[1]
    )
    expect(names.length).toBeGreaterThan(40)
    expect(names.map(kebabId).sort()).toEqual([...MODAL_IDS].sort())
    // ModalLayout puts the token next to `modal`.
    expect(read(`${app}managers/ModalsManager/ModalLayout.tsx`)).toContain(
      'identityHook("modal", stackEntry.name)'
    )
  })

  it("give every Settings page an id", () => {
    const paths = [
      ...read(`${app}pages/Settings/index.tsx`).matchAll(
        /path: "(\/settings[^"]*)"/g
      )
    ].map((m) => m[1])
    expect(paths.map(settingsPageId)).toEqual([
      ...GDL_IDENTITIES["settings.nav.item"]
    ])
    expect(settingsPageId("/settings")).toBe("general")
    expect(settingsPageId("/settings/custom-commands")).toBe("custom-commands")
  })

  it("give every Search filter section an id", () => {
    const source = read(`${app}pages/Search/FilterSidebar/index.tsx`)
    const ids = [
      ...source.matchAll(/data-gdl="search\.filter search\.filter\.([a-z-]+)"/g)
    ].map((m) => m[1])
    expect(ids).toEqual([...GDL_IDENTITIES["search.filter"]])
  })

  it("mark the platform logos", () => {
    const source = read(`${app}components/PlatformLogo.tsx`)
    for (const id of GDL_IDENTITIES["platform-logo"]) {
      expect(source).toContain(`data-gdl="platform-logo platform-logo.${id}"`)
    }
  })
})
