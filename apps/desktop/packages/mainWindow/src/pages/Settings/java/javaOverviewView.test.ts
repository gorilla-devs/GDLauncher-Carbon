import { describe, expect, test } from "vitest"
import type {
  FEJavaEntry,
  FEJavaGroup,
  FEJavaOverview,
  FEJavaUsage,
  FEManagedJavaInstall
} from "@gd/core_module/bindings"
import javaEnglish from "../../../../../../../../packages/i18n/locale/english/java.json"
import generalEnglish from "../../../../../../../../packages/i18n/locale/english/general.json"
import {
  addFromFileDialogProperties,
  advisoryMuted,
  customProfileUsers,
  deleteNeedsConfirm,
  deleteTooltip,
  groupHeader,
  installRowsFor,
  javaChoices,
  javasWithAdvisory,
  markerText,
  profileUsers,
  removedUnusedMessage,
  reviewTargets,
  sourceLine,
  unusedJavas
} from "./javaOverviewView"

// Renders keys with the shipped English copy, so assertions read as the UI.
const strings: Record<string, string> = {
  ...Object.fromEntries(
    Object.entries(javaEnglish).map(([key, value]) => [`java:${key}`, value])
  ),
  "general:_trn_delete": generalEnglish._trn_delete
}
// Plural keys resolve the way i18next does in English: `_one` for 1.
const t = (key: string, options: Record<string, string | number> = {}) => {
  const plural =
    typeof options.count === "number"
      ? `${key}_${options.count === 1 ? "one" : "other"}`
      : undefined
  const template = strings[key] ?? (plural ? strings[plural] : undefined)
  if (template === undefined) throw new Error(`missing string ${key}`)
  return template.replace(/{{(\w+)}}/g, (_, name: string) =>
    String(options[name])
  )
}
const formatBytes = (bytes: number) => `${bytes / 1024 / 1024} MB`

const group = (over: Partial<FEJavaGroup> = {}): FEJavaGroup => ({
  major: 21,
  fromVersion: "1.20.5",
  toVersion: "1.21.11",
  newest: false,
  oldest: false,
  inUseJavaId: null,
  userChosen: false,
  downloadsWhenNeeded: false,
  javas: [],
  ...over
})
const entry = (over: Partial<FEJavaEntry> = {}): FEJavaEntry => ({
  id: "j1",
  path: "/usr/lib/jvm/java-21-openjdk-amd64/bin/java",
  version: "21.0.8",
  major: 21,
  vendorName: "OpenJDK",
  source: "local",
  sizeBytes: null,
  isValid: true,
  nativity: "native",
  pinned: false,
  updateTo: null,
  knownIssues: [],
  usedBy: [],
  runningGame: false,
  pendingRemoval: false,
  ...over
})
const install = (
  over: Partial<FEManagedJavaInstall> = {}
): FEManagedJavaInstall => ({
  buildId: "b1",
  vendor: "azul",
  major: 21,
  version: "21.0.13.1",
  progress: "idle",
  ...over
})

describe("groupHeader", () => {
  test("uses the range, with open ends for the newest and oldest majors", () => {
    expect(groupHeader(group(), t)).toBe("Java 21 · Minecraft 1.20.5 – 1.21.11")
    expect(
      groupHeader(
        group({ major: 25, fromVersion: "26.1", toVersion: "", newest: true }),
        t
      )
    ).toBe("Java 25 · Minecraft 26.1 and newer")
    expect(
      groupHeader(
        group({
          major: 8,
          fromVersion: "1.0",
          toVersion: "1.16.5",
          oldest: true
        }),
        t
      )
    ).toBe("Java 8 · Minecraft 1.16.5 and older")
  })

  test("names a single version once, and only the major without version data", () => {
    expect(
      groupHeader(
        group({ major: 16, fromVersion: "1.17", toVersion: "1.17" }),
        t
      )
    ).toBe("Java 16 · Minecraft 1.17")
    expect(groupHeader(group({ fromVersion: "", toVersion: "" }), t)).toBe(
      "Java 21"
    )
    expect(
      groupHeader(group({ fromVersion: "", toVersion: "", newest: true }), t)
    ).toBe("Java 21")
  })
})

describe("markerText", () => {
  test("names the Minecraft range, with the Java in the accessible name", () => {
    expect(markerText(group(), "Azul Zulu 21.0.13.1", true, t)).toEqual({
      tooltip: "Automatic picks this for Minecraft 1.20.5 – 1.21.11",
      label:
        "Automatic picks Azul Zulu 21.0.13.1 for Minecraft 1.20.5 – 1.21.11"
    })
    expect(
      markerText(
        group({
          major: 8,
          fromVersion: "1.0",
          toVersion: "1.16.5",
          oldest: true
        }),
        "OpenJDK 1.8.0_492",
        false,
        t
      )
    ).toEqual({
      tooltip: "Make Automatic pick this for Minecraft 1.16.5 and older",
      label:
        "Make Automatic pick OpenJDK 1.8.0_492 for Minecraft 1.16.5 and older"
    })
  })

  test("falls back to the Java version without version data", () => {
    const offline = group({ fromVersion: "", toVersion: "" })
    expect(markerText(offline, "OpenJDK 21.0.8", false, t)).toEqual({
      tooltip:
        "Make Automatic pick this for Minecraft versions that need Java 21",
      label:
        "Make Automatic pick OpenJDK 21.0.8 for Minecraft versions that need Java 21"
    })
    expect(markerText(offline, "OpenJDK 21.0.8", true, t)).toEqual({
      tooltip: "Automatic picks this for Minecraft versions that need Java 21",
      label:
        "Automatic picks OpenJDK 21.0.8 for Minecraft versions that need Java 21"
    })
  })
})

describe("advisoryMuted", () => {
  const user = {
    instanceId: 1,
    name: "Survival",
    direct: true,
    viaProfile: null
  }

  test("mutes a found or added Java nothing uses, never GDLauncher's own", () => {
    expect(advisoryMuted(entry({ source: "local" }), true)).toBe(true)
    expect(advisoryMuted(entry({ source: "custom" }), true)).toBe(true)
    expect(
      advisoryMuted(entry({ source: "local", usedBy: [user] }), true)
    ).toBe(false)
    // Auto-update fixes GDLauncher's Javas, so they stay loud.
    expect(advisoryMuted(entry({ source: "managed" }), true)).toBe(false)
  })

  test("mutes nothing while what the instances use is still being read", () => {
    expect(advisoryMuted(entry({ source: "local" }), false)).toBe(false)
  })
})

describe("profileUsers", () => {
  const usage = (
    name: string,
    viaProfile: string | null = null,
    instanceId = name.length
  ): FEJavaUsage => ({
    instanceId,
    name,
    direct: viaProfile !== null,
    viaProfile
  })

  test("lists instances, then counts each custom profile's instances once", () => {
    expect(
      profileUsers([
        usage("Create", "Low-RAM server pack"),
        usage("Survival"),
        usage("Skyblock", "Low-RAM server pack", 9),
        usage("Builders", "Shaders")
      ])
    ).toEqual({
      instances: [usage("Survival")],
      profiles: [
        { name: "Low-RAM server pack", count: 2 },
        { name: "Shaders", count: 1 }
      ]
    })
    expect(profileUsers([])).toEqual({ instances: [], profiles: [] })
  })
})

describe("sourceLine", () => {
  test("GDLauncher's Javas show their size, or nothing when it's unknown", () => {
    expect(
      sourceLine(
        entry({ source: "managed", sizeBytes: 49 * 1024 * 1024 }),
        t,
        formatBytes
      )
    ).toEqual({ text: "Installed by GDLauncher · 49 MB" })
    expect(
      sourceLine(entry({ source: "managed", sizeBytes: null }), t, formatBytes)
    ).toEqual({ text: "Installed by GDLauncher" })
  })

  test("found and added Javas show their path", () => {
    expect(sourceLine(entry(), t, formatBytes)).toEqual({
      text: "On this computer",
      path: "/usr/lib/jvm/java-21-openjdk-amd64/bin/java"
    })
    expect(
      sourceLine(
        entry({ source: "custom", path: "/opt/graalvm-21/bin/java" }),
        t,
        formatBytes
      )
    ).toEqual({ text: "Added by you", path: "/opt/graalvm-21/bin/java" })
  })
})

describe("installRowsFor", () => {
  test("keeps the installs of that major that aren't listed yet", () => {
    const installs = [
      install({ buildId: "new", version: "21.0.13.1" }),
      install({ buildId: "listed", version: "21.0.8" }),
      install({ buildId: "trailing-zero", version: "21.0.9.0" }),
      install({ buildId: "other-major", major: 17, version: "17.0.21" })
    ]
    const entries = [
      entry({ source: "managed", version: "21.0.8" }),
      entry({ id: "j2", source: "managed", version: "21.0.9" })
    ]

    expect(installRowsFor(21, installs, entries).map((i) => i.buildId)).toEqual(
      ["new"]
    )
    expect(installRowsFor(17, installs, entries).map((i) => i.buildId)).toEqual(
      ["other-major"]
    )
  })

  test("a Java found on the computer doesn't hide an install of its version", () => {
    const installs = [install({ version: "21.0.8" })]
    expect(
      installRowsFor(21, installs, [entry({ version: "21.0.8" })])
    ).toEqual(installs)
  })
})

describe("deleteTooltip", () => {
  test("says what deleting does for each source, and nothing for found Javas", () => {
    expect(deleteTooltip(entry(), t, formatBytes)).toBeNull()
    expect(
      deleteTooltip(
        entry({ source: "managed", sizeBytes: 48 * 1024 * 1024 }),
        t,
        formatBytes
      )
    ).toBe("Delete · frees 48 MB")
    expect(deleteTooltip(entry({ source: "managed" }), t, formatBytes)).toBe(
      "Delete"
    )
    expect(deleteTooltip(entry({ source: "custom" }), t, formatBytes)).toBe(
      "Remove from the list"
    )
  })

  test("explains why it can't delete right now", () => {
    expect(
      deleteTooltip(
        entry({ source: "managed", runningGame: true }),
        t,
        formatBytes
      )
    ).toBe("Close the game first")
    expect(
      deleteTooltip(
        entry({ source: "managed", runningGame: true, pendingRemoval: true }),
        t,
        formatBytes
      )
    ).toBe("Removed when the game closes")
  })
})

const user = (name: string, direct = false): FEJavaUsage => ({
  instanceId: name.length,
  name,
  direct,
  viaProfile: null
})

describe("deleteNeedsConfirm", () => {
  test("asks first only when instances use the Java", () => {
    expect(deleteNeedsConfirm(entry({ source: "managed" }))).toBe(false)
    expect(
      deleteNeedsConfirm(
        entry({ source: "managed", usedBy: [user("Vanilla 1.21")] })
      )
    ).toBe(true)
    expect(
      deleteNeedsConfirm(
        entry({ source: "custom", usedBy: [user("Fabric", true)] })
      )
    ).toBe(true)
  })
})

const overview = (over: Partial<FEJavaOverview> = {}): FEJavaOverview => ({
  groups: [
    group({
      javas: [
        entry({ id: "m21-new", source: "managed" }),
        entry({ id: "l21" }),
        entry({ id: "m21-old", source: "managed" })
      ]
    }),
    group({
      major: 17,
      javas: [entry({ id: "m17", major: 17, source: "managed" })]
    })
  ],
  otherJavas: [entry({ id: "c11", major: 11, source: "custom" })],
  unusedJavaIds: [],
  unusedSizeBytes: 0,
  customProfileUsers: {},
  usageComplete: true,
  ...over
})

describe("javasWithAdvisory", () => {
  const issue = (advisoryId: string) => ({
    advisoryId,
    name: advisoryId,
    url: "u",
    fixedVersion: null,
    fix: null,
    mitigated: false
  })

  test("lists the Javas an advisory affects in the list's order", () => {
    const o = overview({
      groups: [
        group({
          javas: [
            entry({ id: "m21", knownIssues: [issue("B")] }),
            entry({ id: "l21", knownIssues: [issue("A"), issue("B")] })
          ]
        }),
        group({
          major: 17,
          javas: [entry({ id: "m17", major: 17, knownIssues: [issue("A")] })]
        })
      ],
      otherJavas: [entry({ id: "c11", major: 11, knownIssues: [issue("A")] })]
    })

    expect(javasWithAdvisory(o, "A")).toEqual(["l21", "m17", "c11"])
    expect(javasWithAdvisory(o, "C")).toEqual([])
  })
})

describe("reviewTargets", () => {
  const issue = (advisoryId: string) => ({
    advisoryId,
    name: advisoryId,
    url: "u",
    fixedVersion: null,
    fix: null,
    mitigated: false
  })
  const used = {
    usedBy: [
      { instanceId: 1, name: "Survival", direct: true, viaProfile: null }
    ]
  }
  const o = overview({
    groups: [
      group({
        javas: [
          entry({ id: "m21", source: "managed", knownIssues: [issue("A")] }),
          entry({ id: "l21", ...used, knownIssues: [issue("A")] }),
          // Found, and nothing uses it any more: its icon is muted.
          entry({ id: "q21", knownIssues: [issue("A")] }),
          entry({ id: "u21", ...used })
        ]
      }),
      group({
        major: 17,
        javas: [
          entry({ id: "z17", major: 17, ...used, knownIssues: [issue("A")] })
        ]
      })
    ],
    otherJavas: []
  })

  test("lands on the Javas the toast counted, in the list's order", () => {
    // m21 is flagged but wasn't counted; u21 was counted but is fixed now.
    expect(reviewTargets(o, "A", ["z17", "l21", "u21"], false)).toEqual([
      "l21",
      "z17"
    ])
  })

  test("falls back to every Java with the advisory when none it counted is listed", () => {
    expect(reviewTargets(o, "A", ["gone"], false)).toEqual([
      "m21",
      "l21",
      "z17"
    ])
    expect(reviewTargets(o, "A", [], false)).toEqual(["m21", "l21", "z17"])
  })

  test("skips a found Java nothing uses, even when the toast counted it", () => {
    expect(reviewTargets(o, "A", ["q21", "l21"], false)).toEqual(["l21"])
    expect(reviewTargets(o, "A", ["q21"], false)).toEqual(["m21", "l21", "z17"])
  })

  test("skips nothing before the instance scan says what is used", () => {
    const scanning = { ...o, usageComplete: false }
    expect(reviewTargets(scanning, "A", ["q21", "l21"], false)).toEqual([
      "l21",
      "q21"
    ])
  })

  test("waits for fresh data when stale data lists none it counted", () => {
    expect(reviewTargets(o, "A", ["new"], true)).toBeUndefined()
    expect(reviewTargets(o, "B", [], true)).toBeUndefined()
    expect(reviewTargets(o, "A", ["z17", "new"], true)).toEqual(["z17"])
  })
})

describe("unusedJavas", () => {
  test("lists the unused Javas in the list's order, skipping ids it no longer has", () => {
    const o = overview({ unusedJavaIds: ["m17", "gone", "m21-old"] })

    expect(unusedJavas(o).map((e) => e.id)).toEqual(["m21-old", "m17"])
    expect(unusedJavas(overview())).toEqual([])
  })
})

describe("javaChoices", () => {
  test("offers every Java once, grouped ones first", () => {
    const o = overview()
    // A Java a group repeats is still offered once.
    o.groups[1].javas.push(o.groups[0].javas[1])

    expect(javaChoices(o).map((e) => e.id)).toEqual([
      "m21-new",
      "l21",
      "m21-old",
      "m17",
      "c11"
    ])
  })
})

describe("customProfileUsers", () => {
  test("counts a profile's instances, and is unknown without the overview", () => {
    const o = overview({ customProfileUsers: { "Modded fast": 3 } })

    expect(customProfileUsers(o, "Modded fast")).toBe(3)
    expect(customProfileUsers(o, "Old Forge")).toBe(0)
    // Loading, or it failed: deleting asks rather than guess 0.
    expect(customProfileUsers(undefined, "Modded fast")).toBeNull()
  })
})

describe("addFromFileDialogProperties", () => {
  test("picks files and folders on macOS, and the binary elsewhere", () => {
    expect(addFromFileDialogProperties("MacIntel")).toEqual([
      "openFile",
      "openDirectory"
    ])
    // Windows and Linux show a folder picker for both, which hides binaries.
    expect(addFromFileDialogProperties("Win32")).toEqual(["openFile"])
    expect(addFromFileDialogProperties("Linux x86_64")).toEqual(["openFile"])
  })
})

describe("removedUnusedMessage", () => {
  test("says what was removed, or that it waits for the game", () => {
    expect(removedUnusedMessage({ count: 2, deferred: false }, t)).toBe(
      "Removed 2 Javas"
    )
    expect(removedUnusedMessage({ count: 1, deferred: false }, t)).toBe(
      "Removed 1 Java"
    )
    expect(removedUnusedMessage({ count: 3, deferred: true }, t)).toBe(
      "3 Javas will be removed when the game closes"
    )
    expect(removedUnusedMessage({ count: 1, deferred: true }, t)).toBe(
      "1 Java will be removed when the game closes"
    )
  })
})
