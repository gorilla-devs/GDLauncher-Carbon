import { describe, expect, test, vi } from "vitest"
import type { FEJavaEntry, FEJavaProfile } from "@gd/core_module/bindings"
import javaEnglish from "../../../../../../../../../../packages/i18n/locale/english/java.json"
import {
  automaticIfStillMismatched,
  buildJavaOptions,
  currentOption,
  isSystemProfile,
  javaHomeName,
  javaNeed,
  misfitNote,
  optionKey,
  optionLabel,
  overrideFor,
  type JavaNeed,
  type JavaOption,
  type StoredJavaOption
} from "./javaSelectOptions"

// Renders keys with the shipped English copy, so assertions read as the UI.
const t = (key: string, options: Record<string, string | number> = {}) => {
  const template = (javaEnglish as Record<string, string>)[
    key.replace(/^java:/, "")
  ]
  if (template === undefined) throw new Error(`missing string ${key}`)
  return template.replace(/{{(\w+)}}/g, (_, name: string) =>
    String(options[name])
  )
}

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
const profile = (over: Partial<FEJavaProfile> = {}): FEJavaProfile => ({
  name: "Low-RAM server pack",
  javaId: "z17",
  isSystem: false,
  ...over
})

const DELTA = "__gdl_system_java_profile__delta"
const GAMMA = "__gdl_system_java_profile__gamma"

const need = (major: number, accepted = [major], below: string | null = null) =>
  ({ major, below, accepted, floor: null }) satisfies JavaNeed
const need21 = need(21)
const forge = need(8, [8], "8.0.312")
const caves = need(17, [16, 17])
// A mod on Minecraft 1.20.1 (Java 17) crashed asking for Java 21.
const floor21 = { ...need(21), floor: 21 } satisfies JavaNeed

const azul21 = entry({
  id: "z21",
  path: "/gdl/managed_javas/zulu21.0.13.1/bin/java",
  version: "21.0.13.1",
  vendorName: "Azul Zulu",
  source: "managed"
})
const mojang21 = entry({
  id: "m21",
  path: "/gdl/managed_javas/mojang-delta-21.0.7/bin/java",
  version: "21.0.7",
  vendorName: "Mojang",
  source: "managed"
})
const found21 = entry({ id: "l21" })
const azul25 = entry({
  id: "z25",
  path: "/gdl/managed_javas/zulu25.0.5.1/bin/java",
  version: "25.0.5.1",
  major: 25,
  vendorName: "Azul Zulu",
  source: "managed"
})
const azul17 = entry({
  id: "z17",
  path: "/gdl/managed_javas/zulu17.0.21.1/bin/java",
  version: "17.0.21.1",
  major: 17,
  vendorName: "Azul Zulu",
  source: "managed"
})
const old17 = entry({
  id: "z17-old",
  path: "/gdl/managed_javas/zulu17.0.20/bin/java",
  version: "17.0.20",
  major: 17,
  vendorName: "Azul Zulu",
  source: "managed"
})
const java8 = entry({
  id: "l8",
  path: "/usr/lib/jvm/java-8/bin/java",
  version: "8.0.492",
  major: 8
})
const azul8 = entry({
  id: "z8",
  path: "/gdl/managed_javas/zulu8.0.302/bin/java",
  version: "8.0.302",
  major: 8,
  vendorName: "Azul Zulu",
  source: "managed"
})
const java16 = entry({
  id: "l16",
  path: "/usr/lib/jvm/java-16/bin/java",
  version: "16.0.2",
  major: 16
})
const java11 = entry({
  id: "l11",
  path: "/usr/lib/jvm/java-11/bin/java",
  version: "11.0.20",
  major: 11
})
const entries = [java8, old17, mojang21, azul17, found21, azul25, azul21]

/** Each section as its header and option keys, for readable assertions. */
const outline = (sections: { header?: string; options: JavaOption[] }[]) =>
  sections.map((s) => [s.header ?? null, s.options.map(optionKey)])

/** Whether each listed Java fits, by id. */
const fits = (sections: { options: JavaOption[] }[]) =>
  Object.fromEntries(
    sections
      .flatMap((s) => s.options)
      .flatMap((o) => (o.kind === "java" ? [[o.entry.id, o.fits]] : []))
  )

describe("buildJavaOptions", () => {
  test("orders Automatic, the needed Java, other versions, profiles, then a file", () => {
    const { sections } = buildJavaOptions(
      need21,
      "1.21.4",
      entries,
      [profile(), profile({ name: GAMMA, javaId: "z17", isSystem: true })],
      { major: 21, below: null },
      t
    )

    expect(outline(sections)).toEqual([
      [null, ["automatic"]],
      [
        "Java 21 · Minecraft 1.21.4",
        ["java:z21", "java:l21", "java:m21", "download:21"]
      ],
      [
        "Other Java versions",
        ["java:z25", "java:z17", "java:z17-old", "java:l8"]
      ],
      ["Profiles", ["profile:Low-RAM server pack"]],
      [null, ["file"]]
    ])
    // The profile names its Java's major, which doesn't fit 1.21.4.
    expect(sections[3].options[0]).toEqual({
      kind: "profile",
      name: "Low-RAM server pack",
      major: 17,
      fits: false
    })
  })

  test("marks every major the instance doesn't run on, older or newer", () => {
    const { sections } = buildJavaOptions(
      need21,
      "1.21.4",
      entries,
      [],
      null,
      t
    )

    // What the core would call a mismatch, and nothing else.
    expect(fits(sections)).toEqual({
      z21: true,
      l21: true,
      m21: true,
      z25: false,
      z17: false,
      "z17-old": false,
      l8: false
    })
  })

  test("puts Forge 1.16.5's capped builds after the ones that fit", () => {
    const { sections } = buildJavaOptions(
      forge,
      "1.16.5",
      [java8, azul8, azul17],
      [],
      null,
      t
    )

    // Newest first would put 8.0.492 on top; it can't run Forge 1.16.5. The
    // header names the cap, and Forge rather than Minecraft.
    expect(outline(sections)[1]).toEqual([
      "Java 8 older than 8.0.312 · Forge 1.16.5",
      ["java:z8", "java:l8"]
    ])
    expect(fits(sections)).toEqual({ z8: true, l8: false, z17: false })
  })

  test("offers Download Java N for what Automatic would download, only when nothing that fits is there", () => {
    // Forge 1.16.5 with only a Java 8 too new for it: Automatic downloads a
    // build below the cap.
    const forgeOptions = buildJavaOptions(
      forge,
      "1.16.5",
      [java8],
      [],
      { major: 8, below: "8.0.312" },
      t
    ).sections[1].options
    expect(forgeOptions.map(optionKey)).toEqual(["java:l8", "download:8"])
    expect(forgeOptions[1]).toEqual({
      kind: "download",
      major: 8,
      below: "8.0.312"
    })

    // 1.17 where no Java 16 downloads: Automatic gets Java 17.
    expect(
      outline(
        buildJavaOptions(
          caves,
          "1.17.1",
          [java11],
          [],
          { major: 17, below: null },
          t
        ).sections
      )[1]
    ).toEqual(["Java 16 or 17 · Minecraft 1.17.1", ["download:17"]])
  })

  test("lists every Java from an instance's Java floor up as fitting", () => {
    const { sections } = buildJavaOptions(
      floor21,
      "1.20.1",
      [azul17, found21, azul25, java8],
      [],
      null,
      t
    )

    expect(outline(sections).slice(1, 3)).toEqual([
      ["Java 21 · Minecraft 1.20.1", ["java:z25", "java:l21"]],
      ["Other Java versions", ["java:z17", "java:l8"]]
    ])
    expect(fits(sections)).toEqual({
      z25: true,
      l21: true,
      z17: false,
      l8: false
    })
  })

  test("lists Java 16 with 17 for Minecraft 1.17, which runs on either", () => {
    const { sections } = buildJavaOptions(
      caves,
      "1.17",
      [java16, azul17, java11],
      [],
      null,
      t
    )

    // The header names both, so a Java 17 under it reads as fitting.
    expect(outline(sections).slice(1, 3)).toEqual([
      ["Java 16 or 17 · Minecraft 1.17", ["java:z17", "java:l16"]],
      ["Other Java versions", ["java:l11"]]
    ])
    expect(fits(sections)).toEqual({ z17: true, l16: true, l11: false })
  })

  test("marks the custom profiles whose Java doesn't fit, as a launch sees them", () => {
    const profiles = [
      profile({ name: "Old", javaId: "z17" }),
      profile({ name: "Right", javaId: "z21" }),
      profile({ name: "Empty", javaId: null }),
      profile({ name: "Broken", javaId: "bad" })
    ]
    const withBroken = [
      ...entries,
      entry({ id: "bad", major: 17, isValid: false })
    ]
    const { sections } = buildJavaOptions(
      need21,
      "1.21.4",
      withBroken,
      profiles,
      null,
      t
    )
    const listed = sections
      .flatMap((s) => s.options)
      .filter((o) => o.kind === "profile")

    // A profile with no Java that starts runs on Automatic, which fits.
    expect(listed).toEqual([
      { kind: "profile", name: "Old", major: 17, fits: false },
      { kind: "profile", name: "Right", major: 21, fits: true },
      { kind: "profile", name: "Empty", major: undefined, fits: true },
      { kind: "profile", name: "Broken", major: undefined, fits: true }
    ])
  })

  test("leaves out empty sections and profiles without custom ones", () => {
    const { sections } = buildJavaOptions(
      need(17),
      "1.20.4",
      [azul17],
      [profile({ name: GAMMA, isSystem: true })],
      null,
      t
    )

    expect(outline(sections)).toEqual([
      [null, ["automatic"]],
      ["Java 17 · Minecraft 1.20.4", ["java:z17"]],
      [null, ["file"]]
    ])
    // Nothing installed for the needed Java, and nothing to download.
    expect(
      outline(buildJavaOptions(need(17), "1.20.4", [], [], null, t).sections)
    ).toEqual([
      [null, ["automatic"]],
      [null, ["file"]]
    ])
  })

  test("lists every Java under no header when the needed Java is unknown", () => {
    const { sections } = buildJavaOptions(
      null,
      "99.1",
      [azul17, azul21],
      [],
      null,
      t
    )

    expect(outline(sections)).toEqual([
      [null, ["automatic"]],
      [null, ["java:z21", "java:z17"]],
      [null, ["file"]]
    ])
    expect(fits(sections)).toEqual({ z21: true, z17: true })
  })
})

describe("javaNeed", () => {
  test("reads the instance details", () => {
    expect(
      javaNeed({
        requiredJavaMajor: 17,
        requiredJavaBelow: null,
        acceptedJavaMajors: [16, 17],
        javaFloor: null
      })
    ).toEqual(caves)
    expect(
      javaNeed({
        requiredJavaMajor: 8,
        requiredJavaBelow: "8.0.312",
        acceptedJavaMajors: [8],
        javaFloor: null
      })
    ).toEqual(forge)
    expect(
      javaNeed({
        requiredJavaMajor: 21,
        requiredJavaBelow: null,
        acceptedJavaMajors: [21],
        javaFloor: 21
      })
    ).toEqual(floor21)
    expect(
      javaNeed({
        requiredJavaMajor: null,
        requiredJavaBelow: null,
        acceptedJavaMajors: [],
        javaFloor: null
      })
    ).toBeNull()
  })
})

describe("misfitNote", () => {
  test("says why a Java doesn't fit, or nothing when it does", () => {
    // Another major, older or newer.
    expect(misfitNote(azul17, need21, "1.21.4", t)).toBe(
      "Minecraft 1.21.4 needs Java 21"
    )
    expect(misfitNote(azul25, need21, "1.21.4", t)).toBe(
      "Minecraft 1.21.4 needs Java 21"
    )
    // Forge 1.16.5 needs a capped Java 8, whatever the Java is.
    expect(misfitNote(azul17, forge, "1.16.5", t)).toBe(
      "Forge 1.16.5 needs Java 8 older than 8.0.312"
    )
    // At or above the cap.
    expect(misfitNote(java8, forge, "1.16.5", t)).toBe(
      "Forge 1.16.5 needs Java 8 older than 8.0.312"
    )
    expect(misfitNote(azul8, forge, "1.16.5", t)).toBeUndefined()
    expect(misfitNote(java16, caves, "1.17", t)).toBeUndefined()
    expect(misfitNote(azul17, null, "99.1", t)).toBeUndefined()
    // Below a Java floor: it's the instance, not Minecraft, that needs more.
    expect(misfitNote(azul17, floor21, "1.20.1", t)).toBe(
      "This instance needs Java 21 or later"
    )
    expect(misfitNote(azul25, floor21, "1.20.1", t)).toBeUndefined()
  })
})

describe("optionLabel", () => {
  test("is the name each option shows, for typeahead", () => {
    const labels = [
      { kind: "automatic" },
      { kind: "java", entry: azul21, fits: true },
      { kind: "java", entry: azul17, fits: false },
      { kind: "download", major: 21, below: null },
      { kind: "profile", name: "Low-RAM server pack", major: 17 },
      { kind: "file" }
    ].map((option) => optionLabel(option as JavaOption, need21, t))

    expect(labels).toEqual([
      "Automatic",
      "Azul Zulu 21.0.13.1",
      "Java 17 · Azul Zulu 17.0.21.1",
      "Download Java 21",
      "Low-RAM server pack",
      "Choose a file"
    ])
    // Java 16 shares 1.17's group, so it names its major.
    expect(
      optionLabel({ kind: "java", entry: java16, fits: true }, caves, t)
    ).toBe("Java 16 · OpenJDK 16.0.2")
  })
})

describe("automaticIfStillMismatched", () => {
  const client = (mismatch: boolean | null) => ({
    details: vi.fn(async () =>
      mismatch === null ? null : { javaOverrideMismatch: mismatch }
    ),
    setAutomatic: vi.fn(async () => null)
  })

  test("moves that instance to Automatic only while its Java still doesn't fit", async () => {
    const wrong = client(true)
    expect(await automaticIfStillMismatched(7, wrong)).toBe(true)
    expect(wrong.details).toHaveBeenCalledWith(7)
    expect(wrong.setAutomatic).toHaveBeenCalledWith(7)

    for (const state of [false, null]) {
      const fine = client(state)
      expect(await automaticIfStillMismatched(7, fine)).toBe(false)
      expect(fine.setAutomatic).not.toHaveBeenCalled()
    }
  })

  test("leaves the failure to the caller", async () => {
    const failing = {
      details: vi.fn(async () => {
        throw new Error("offline")
      }),
      setAutomatic: vi.fn(async () => null)
    }

    await expect(automaticIfStillMismatched(7, failing)).rejects.toThrow(
      "offline"
    )
    expect(failing.setAutomatic).not.toHaveBeenCalled()
  })
})

describe("currentOption", () => {
  test("reads no override, the required profile and unset overrides as Automatic", () => {
    for (const override of [
      null,
      { Profile: DELTA },
      { Profile: null },
      { Path: null }
    ]) {
      expect(currentOption(override, DELTA, entries)).toEqual({
        kind: "automatic"
      })
    }
  })

  test("finds a known path's Java", () => {
    expect(currentOption({ Path: azul17.path }, DELTA, entries)).toEqual({
      kind: "java",
      entry: azul17,
      fits: true
    })
  })

  test("names an unknown path after its Java's folder", () => {
    const option = currentOption(
      { Path: "/opt/graalvm-21/bin/java" },
      DELTA,
      entries
    )

    expect(option.kind).toBe("java")
    if (option.kind !== "java") return
    expect(option.entry.path).toBe("/opt/graalvm-21/bin/java")
    expect(option.entry.vendorName).toBe("graalvm-21")
    expect(option.entry.version).toBe("")
    expect(optionKey(option)).toBe("path:/opt/graalvm-21/bin/java")
  })

  test("keeps a custom profile, with its Java's major", () => {
    expect(
      currentOption({ Profile: "Low-RAM server pack" }, DELTA, entries, [
        profile()
      ])
    ).toEqual({ kind: "profile", name: "Low-RAM server pack", major: 17 })
    // Before the profiles load, the profile still shows.
    expect(
      currentOption({ Profile: "Low-RAM server pack" }, DELTA, entries)
    ).toEqual({
      kind: "profile",
      name: "Low-RAM server pack",
      major: undefined
    })
  })

  test("reads a profile a launch ignores as Automatic", () => {
    const profiles = [
      profile({ name: "Empty", javaId: null }),
      profile({ name: "Dangling", javaId: "gone" }),
      profile({ name: "Broken", javaId: "bad" })
    ]
    const withBroken = [...entries, entry({ id: "bad", isValid: false })]

    for (const name of ["Deleted", "Empty", "Dangling", "Broken"]) {
      expect(
        currentOption({ Profile: name }, DELTA, withBroken, profiles)
      ).toEqual({ kind: "automatic" })
    }
  })

  test("keeps another Java version's system profile", () => {
    expect(
      currentOption({ Profile: GAMMA }, DELTA, entries, [
        profile({ name: GAMMA, javaId: "z17", isSystem: true })
      ])
    ).toEqual({ kind: "profile", name: GAMMA, major: 17 })
  })
})

describe("overrideFor", () => {
  test("round-trips through currentOption", () => {
    const options: StoredJavaOption[] = [
      { kind: "automatic" },
      { kind: "java", entry: azul21, fits: true },
      { kind: "java", entry: java8, fits: false },
      { kind: "profile", name: "Low-RAM server pack", major: 17 }
    ]

    for (const option of options) {
      const back = currentOption(overrideFor(option), DELTA, entries, [
        profile()
      ])
      expect(optionKey(back)).toBe(optionKey(option))
    }
    expect(overrideFor({ kind: "automatic" })).toBeNull()
    expect(overrideFor({ kind: "java", entry: azul21, fits: true })).toEqual({
      Path: azul21.path
    })
    expect(
      overrideFor({ kind: "profile", name: "Low-RAM server pack" })
    ).toEqual({ Profile: "Low-RAM server pack" })
  })

  test("an unknown path keeps its override", () => {
    const option = currentOption(
      { Path: "C:\\Java\\jdk-21\\bin\\java.exe" },
      DELTA,
      entries
    )
    expect(overrideFor(option)).toEqual({
      Path: "C:\\Java\\jdk-21\\bin\\java.exe"
    })
  })
})

describe("javaHomeName", () => {
  test("names the Java's home folder rather than the binary", () => {
    expect(javaHomeName("/opt/graalvm-21/bin/java")).toBe("graalvm-21")
    expect(javaHomeName("C:\\Program Files\\Java\\jdk-21\\bin\\java.exe")).toBe(
      "jdk-21"
    )
    expect(
      javaHomeName(
        "/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java"
      )
    ).toBe("zulu-21.jdk")
    expect(javaHomeName("/usr/bin/java")).toBe("/usr/bin/java")
    expect(javaHomeName("java")).toBe("java")
  })
})

describe("isSystemProfile", () => {
  test("tells the launcher's profiles from custom ones", () => {
    expect(isSystemProfile(GAMMA)).toBe(true)
    expect(isSystemProfile("Low-RAM server pack")).toBe(false)
  })
})
