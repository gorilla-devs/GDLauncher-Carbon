import { describe, expect, it } from "vitest"
import i18next from "i18next"
import type {
  FESkippedDependencyReason,
  FETaskWarnings
} from "@gd/core_module/bindings"
import english from "../../../../../../../packages/i18n/locale/english/notifications.json"
import { describeTaskWarnings, type Translate } from "./describe"

const t: Translate = (key, options) => {
  if (!options) return key
  const { interpolation: _, ...rest } = options
  return `${key} ${JSON.stringify(rest)}`
}

const skipped = (
  reason: FESkippedDependencyReason,
  name: string | null = "Sodium",
  project_id = "AANobbMI",
  platform: "Curseforge" | "Modrinth" = "Modrinth"
) =>
  ({
    type: "SkippedDependency",
    value: { name, platform, project_id, reason }
  }) as const

const replaced = (pinned: string, installed: string) =>
  ({
    type: "ReplacedPinnedVersion",
    value: { name: "Sodium", project_id: "AANobbMI", pinned, installed }
  }) as const

const label = (name: string, project: string) =>
  `notifications:_trn_addon_dependency_label ${JSON.stringify({ name, project })}`

describe("describeTaskWarnings", () => {
  it("names the addon, its instance and every skipped dependency with its reason", () => {
    const entry: FETaskWarnings = {
      id: 7,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "Translations for Sodium", instance_name: "Vanilla" }
      },
      subject: null,
      warnings: [
        skipped("NoCompatibleVersion"),
        skipped("NotDownloadable", "Private Library", "9", "Curseforge")
      ]
    }
    expect(describeTaskWarnings(entry, t)).toEqual({
      title:
        'notifications:_trn_addon_dependencies_skipped_in_instance {"title":"Translations for Sodium","instance":"Vanilla"}',
      lines: [
        `notifications:_trn_addon_dependency_skipped_no_compatible_version ${JSON.stringify({ dependency: label("Sodium", "AANobbMI") })}`,
        `notifications:_trn_addon_dependency_skipped_not_downloadable ${JSON.stringify({ dependency: label("Private Library", "9") })}`
      ],
      hint: 'notifications:_trn_addon_dependencies_skipped_hint {"title":"Translations for Sodium"}'
    })
  })

  it("names a dependency the platform no longer knows by platform and id, and a file only once", () => {
    const entry: FETaskWarnings = {
      id: 9,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "Parent", instance_name: "Pack" }
      },
      subject: null,
      warnings: [
        skipped("Unavailable", null, "12345", "Curseforge"),
        skipped("Unavailable", null, "AANobbMI", "Modrinth"),
        skipped("Unavailable", "library-1.0.jar", "library-1.0.jar")
      ]
    }
    expect(describeTaskWarnings(entry, t)?.lines).toEqual([
      `notifications:_trn_addon_dependency_skipped_unavailable ${JSON.stringify({ dependency: 'notifications:_trn_addon_dependency_unnamed_curseforge {"project":"12345"}' })}`,
      `notifications:_trn_addon_dependency_skipped_unavailable ${JSON.stringify({ dependency: 'notifications:_trn_addon_dependency_unnamed_modrinth {"project":"AANobbMI"}' })}`,
      `notifications:_trn_addon_dependency_skipped_unavailable ${JSON.stringify({ dependency: "library-1.0.jar" })}`
    ])
  })

  it("falls back to a generic addon name and stays silent without warnings", () => {
    const entry: FETaskWarnings = {
      id: 8,
      name: { translation: "InstanceTaskDeleting" },
      subject: null,
      warnings: [skipped("Unavailable")]
    }
    expect(describeTaskWarnings(entry, t)?.title).toBe(
      'notifications:_trn_addon_dependencies_skipped {"title":"notifications:_trn_addon_fallback_name"}'
    )
    expect(describeTaskWarnings({ ...entry, warnings: [] }, t)).toBeNull()
  })

  it("shows names with apostrophes as written under the app's escaping i18next", async () => {
    // The app keeps i18next's default escapeValue: true.
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const translate: Translate = (key, options) =>
      i18n.t(key as string, options)
    const entry: FETaskWarnings = {
      id: 10,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "Farmer's Delight", instance_name: "Steve's <Pack>" }
      },
      subject: null,
      warnings: [skipped("NoCompatibleVersion", "Biomes O' Plenty", "220318")]
    }
    expect(describeTaskWarnings(entry, translate)).toEqual({
      title:
        "Farmer's Delight was installed in Steve's <Pack> without some of its required dependencies",
      lines: [
        "Biomes O' Plenty (220318): no version for this Minecraft version and mod loader"
      ],
      hint: "If the game does not start, install them yourself or remove Farmer's Delight."
    })
  })
  it("names the addon by its project title, not a version name like 4.0", () => {
    const entry: FETaskWarnings = {
      id: 11,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "4.0", instance_name: "Vanilla" }
      },
      subject: "Cobblemon Emissive Ores",
      warnings: [skipped("NoCompatibleVersion", "Cobblemon", "MdwFAVRL")]
    }
    const toast = describeTaskWarnings(entry, t)
    expect(toast?.title).toBe(
      'notifications:_trn_addon_dependencies_skipped_in_instance {"title":"Cobblemon Emissive Ores","instance":"Vanilla"}'
    )
    expect(toast?.hint).toBe(
      'notifications:_trn_addon_dependencies_skipped_hint {"title":"Cobblemon Emissive Ores"}'
    )
  })

  it("says which pinned version was replaced and by which", async () => {
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const translate: Translate = (key, options) =>
      i18n.t(key as string, options)
    const entry: FETaskWarnings = {
      id: 12,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "Iris 1.11.4", instance_name: "Shaders" }
      },
      subject: "Iris Shaders",
      warnings: [replaced("mc26.1.2-0.9.2", "mc26.1.1-0.8.9-fabric")]
    }
    expect(describeTaskWarnings(entry, translate)).toEqual({
      title:
        "Iris Shaders was installed in Shaders, but not with the dependency versions it asks for",
      lines: [
        "Sodium (AANobbMI): installed mc26.1.1-0.8.9-fabric, because the required mc26.1.2-0.9.2 is not for this Minecraft version and mod loader"
      ],
      hint: "If the game does not start, try another version of Iris Shaders or remove it."
    })
    // A skipped dependency next to it decides the title and the way out.
    const mixed = describeTaskWarnings(
      { ...entry, warnings: [replaced("a", "b"), skipped("Unavailable")] },
      translate
    )
    expect(mixed?.title).toBe(
      "Iris Shaders was installed in Shaders without some of its required dependencies"
    )
    expect(mixed?.lines).toHaveLength(2)
    expect(mixed?.hint).toBe(
      "If the game does not start, install them yourself or remove Iris Shaders."
    )
  })
  it("names the server a server addon was installed on, and why each dependency is missing", async () => {
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const translate: Translate = (key, options) =>
      i18n.t(key as string, options)
    const entry: FETaskWarnings = {
      id: 13,
      name: {
        translation: "ServerTaskInstallMod",
        args: {
          mod_name: "Server addon and required dependencies",
          server_name: "Steve's Create SMP"
        }
      },
      subject: "Create: Steam 'n' Rails",
      warnings: [
        skipped("NotForServers", "Shader Toggle", "toggle"),
        skipped("RemovedDuringInstall", "Create", "LNytGWDc"),
        skipped("NoCompatibleVersion", "Flywheel", "5lpoZ8Ce")
      ]
    }
    expect(describeTaskWarnings(entry, translate)).toEqual({
      title:
        "Create: Steam 'n' Rails was installed on Steve's Create SMP without some of its required dependencies",
      lines: [
        "Shader Toggle (toggle): its authors say it does not run on servers",
        "Create (LNytGWDc): it was removed while the installation was running",
        "Flywheel (5lpoZ8Ce): no version for this Minecraft version and mod loader"
      ],
      hint: "If the server does not start, install them yourself or remove Create: Steam 'n' Rails."
    })
  })

  it("says which required dependencies are installed but disabled, and to enable them", async () => {
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const translate: Translate = (key, options) =>
      i18n.t(key as string, options)
    const entry: FETaskWarnings = {
      id: 17,
      name: {
        translation: "InstanceTaskInstallMod",
        args: { mod_name: "Sodium Extra 0.5", instance_name: "Steve's Pack" }
      },
      subject: "Sodium Extra",
      warnings: [
        skipped("Disabled", "Fabric API", "306612", "Curseforge"),
        skipped("Disabled", "Sodium", "AANobbMI")
      ]
    }
    expect(describeTaskWarnings(entry, translate)).toEqual({
      title:
        "Sodium Extra was installed in Steve's Pack, but some of its required dependencies are disabled",
      lines: [
        "Fabric API (306612): installed, but disabled",
        "Sodium (AANobbMI): installed, but disabled"
      ],
      hint: "Enable them in the instance's Addons tab, or the game may not start with Sodium Extra."
    })
    // Next to one that is missing, the missing one decides the title.
    const mixed = describeTaskWarnings(
      {
        ...entry,
        warnings: [
          skipped("Disabled", "Fabric API", "306612", "Curseforge"),
          skipped("NoCompatibleVersion")
        ]
      },
      translate
    )
    expect(mixed?.title).toBe(
      "Sodium Extra was installed in Steve's Pack without some of its required dependencies"
    )
    expect(mixed?.lines).toEqual([
      "Fabric API (306612): installed, but disabled",
      "Sodium (AANobbMI): no version for this Minecraft version and mod loader"
    ])
    expect(
      describeTaskWarnings(
        {
          ...entry,
          name: { translation: "InstanceTaskDeleting" },
          warnings: [skipped("Disabled", "Fabric API", "306612", "Curseforge")]
        },
        t
      )?.title
    ).toBe(
      'notifications:_trn_addon_dependencies_disabled {"title":"Sodium Extra"}'
    )
  })

  it("says which version was installed when two addons ask for different ones", async () => {
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const translate: Translate = (key, options) =>
      i18n.t(key as string, options)
    const entry: FETaskWarnings = {
      id: 14,
      name: {
        translation: "ServerTaskInstallMod",
        args: { mod_name: "Server addon", server_name: "Cobblemon" }
      },
      subject: "Cobblemon Addon",
      warnings: [
        {
          type: "ConflictingVersions",
          value: {
            name: "Cobblemon",
            platform: "Modrinth",
            project_id: "MdwFAVRL",
            installed: "1.5.2"
          }
        }
      ]
    }
    expect(describeTaskWarnings(entry, translate)).toEqual({
      title:
        "Cobblemon Addon was installed on Cobblemon, but not with the dependency versions it asks for",
      lines: [
        "Cobblemon (MdwFAVRL): the addons ask for different versions of it, 1.5.2 was installed"
      ],
      hint: "If the server does not start, try another version of Cobblemon Addon or remove it."
    })
  })
})
