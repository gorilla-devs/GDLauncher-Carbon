import { readFileSync } from "node:fs"
import { beforeAll, describe, expect, it, vi } from "vitest"
import {
  createI18nOptions,
  i18n,
  languageLocaleCodes,
  loadLanguageFiles
} from "@gd/i18n"
import { isRefusal, refreshWorldsAfter, refusalText } from "./refusalText"

const LANGUAGES = [
  "english",
  "french",
  "italian",
  "japanese",
  "russian",
  "simplified_chinese",
  "spanish"
] as const

// A name i18next would HTML-escape unless told not to.
const FILENAME = "Tom & Jerry's <pack>.zip"
const FOLDER = "Tom & Jerry's <world> (2)"

// Every refusal the core codes for a change to a world or its data packs,
// with the data it sends.
const REFUSALS: [code: string, data?: Record<string, string>][] = [
  ["INSTANCE_GAME_ACTIVE"],
  ["WORLD_GONE"],
  ["WORLD_NOT_A_SAVE", { reason: "NOT_A_FOLDER" }],
  ["WORLD_NOT_A_SAVE", { reason: "INSTALL_INCOMPLETE" }],
  ["WORLD_NOT_A_SAVE", { reason: "NO_LEVEL_DAT" }],
  ["WORLD_NOT_A_SAVE", { reason: "LEVEL_DAT_NOT_A_FILE" }],
  ["WORLD_CHANGED"],
  ["WORLD_CHANGED_DURING_COPY"],
  ["DATAPACK_NAME_TAKEN", { filename: FILENAME }],
  ["DATAPACK_GONE"],
  ["DATAPACK_CHANGED"],
  ["DATAPACK_IN_OTHER_WORLD"],
  ["DATAPACK_FOLDER_NOT_REPLACED"],
  ["NOT_A_DATAPACK_VERSION"],
  ["DATAPACK_NOT_A_ZIP"],
  ["DATAPACK_INVALID", { reason: "NO_PACK_MCMETA" }],
  ["DATAPACK_INVALID", { reason: "METADATA_TOO_LARGE" }],
  ["DATAPACK_INVALID", { reason: "METADATA_NOT_JSON" }],
  ["DATAPACK_INVALID", { reason: "NO_PACK_SECTION" }],
  ["DATAPACK_NOT_DOWNLOADABLE"],
  ["DATAPACK_DOWNLOAD_FAILED", { filename: FILENAME, cause: "timed out" }],
  ["NO_DATAPACK_UPDATE"],
  ["INSTANCE_DATAPACK_GONE"],
  ["INSTANCE_DATAPACK_CHANGED"],
  ["WORLD_NAME_UNREADABLE"],
  ["WORLD_NAME_INVALID"],
  ["WORLD_RENAME_INCOMPLETE", { folder: FOLDER }],
  ["WORLD_NOT_RENAMED"],
  ["WORLD_HOLDS_LINK"],
  ["WORLD_NO_SPACE"],
  ["WORLD_CHANGED_WHILE_COPIED"],
  ["WORLD_BEING_COPIED"]
]
const CORE_TEXT = "The core's English sentence"

beforeAll(async () => {
  await i18n.init(
    createI18nOptions("english", await loadLanguageFiles("english"))
  )
  for (const language of LANGUAGES) {
    const files = await loadLanguageFiles(language)
    for (const [namespace, bundle] of Object.entries(files))
      i18n.addResourceBundle(languageLocaleCodes[language], namespace, bundle)
  }
})

describe("a refusal in the player's language", () => {
  it.each(LANGUAGES)("%s has a sentence for every code", async (language) => {
    await i18n.changeLanguage(languageLocaleCodes.english)
    const english = REFUSALS.map(([code, data]) =>
      refusalText(code, data, CORE_TEXT)
    )
    await i18n.changeLanguage(languageLocaleCodes[language])
    REFUSALS.forEach(([code, data], index) => {
      const text = refusalText(code, data, CORE_TEXT)!
      const where = `${language} ${code}`
      expect(isRefusal(code), where).toBe(true)
      expect(text, where).not.toBe(CORE_TEXT)
      expect(text, where).not.toContain("_trn_")
      expect(text, where).not.toContain("{{")
      if (data?.filename) expect(text, where).toContain(FILENAME)
      if (data?.folder) expect(text, where).toContain(FOLDER)
      if (language !== "english") expect(text, where).not.toBe(english[index])
    })
  })

  it("says in English what the owner approved", async () => {
    await i18n.changeLanguage(languageLocaleCodes.english)
    const said = (code: string, data?: object) => refusalText(code, data)
    expect(said("INSTANCE_GAME_ACTIVE")).toBe(
      "Not available while the game is starting or running."
    )
    expect(said("WORLD_GONE")).toBe("This world no longer exists.")
    expect(said("WORLD_NOT_A_SAVE", { reason: "NO_LEVEL_DAT" })).toBe(
      "GDLauncher can't read this world."
    )
    for (const code of ["WORLD_CHANGED", "WORLD_CHANGED_DURING_COPY"])
      expect(said(code)).toBe("This world changed on disk. Try again.")
    expect(said("DATAPACK_NAME_TAKEN", { filename: FILENAME })).toBe(
      `This world already has a file named “${FILENAME}”.`
    )
    expect(said("DATAPACK_GONE")).toBe(
      "This data pack is no longer in the world."
    )
    for (const code of ["DATAPACK_CHANGED", "INSTANCE_DATAPACK_CHANGED"])
      expect(said(code)).toBe("This data pack changed on disk. Try again.")
    expect(said("DATAPACK_IN_OTHER_WORLD")).toBe(
      "This data pack is in another world."
    )
    expect(said("DATAPACK_FOLDER_NOT_REPLACED")).toBe(
      "A data pack folder can't be replaced. Remove it first."
    )
    expect(said("NOT_A_DATAPACK_VERSION")).toBe(
      "This version is not a data pack."
    )
    expect(said("DATAPACK_NOT_A_ZIP")).toBe(
      "Only a ZIP data pack can be added to a world."
    )
    expect(said("DATAPACK_INVALID", { reason: "NO_PACK_MCMETA" })).toBe(
      "This file is not a valid data pack."
    )
    expect(said("DATAPACK_NOT_DOWNLOADABLE")).toBe(
      "The author doesn't allow launchers to download this file."
    )
    expect(
      said("DATAPACK_DOWNLOAD_FAILED", { filename: FILENAME, cause: "x" })
    ).toBe(`Couldn't download “${FILENAME}”.`)
    expect(said("NO_DATAPACK_UPDATE")).toBe("No newer version is available.")
    expect(said("INSTANCE_DATAPACK_GONE")).toBe(
      "This data pack is no longer in the instance."
    )
    expect(said("NO_DATAPACK_FOR_VERSION", { version: "1.20.1" })).toBe(
      "This project has no data pack for Minecraft 1.20.1."
    )
    expect(said("WORLD_NAME_UNREADABLE")).toBe(
      "GDLauncher can't read this world."
    )
    expect(said("WORLD_NAME_INVALID")).toBe(
      "Enter a name of up to 64 characters."
    )
    expect(said("WORLD_RENAME_INCOMPLETE", { folder: FOLDER })).toBe(
      `The world could not be fully renamed. It is now in the folder “${FOLDER}”.`
    )
    expect(said("WORLD_NOT_RENAMED")).toBe(
      "The world couldn't be renamed. Nothing was changed."
    )
    expect(said("WORLD_HOLDS_LINK")).toBe(
      "This world contains a link to another folder and can't be duplicated."
    )
    expect(said("WORLD_NO_SPACE")).toBe(
      "Not enough free disk space to copy this world."
    )
    expect(said("WORLD_CHANGED_WHILE_COPIED")).toBe(
      "The world changed while it was copied. Nothing was kept; try again."
    )
    expect(said("WORLD_BEING_COPIED")).toBe("This world is being copied.")
  })

  it("falls back to the core's sentence for a code it does not know", () => {
    expect(isRefusal("A_CODE_FROM_A_NEWER_CORE")).toBe(false)
    expect(refusalText("A_CODE_FROM_A_NEWER_CORE", undefined, CORE_TEXT)).toBe(
      CORE_TEXT
    )
    expect(refusalText(undefined, undefined, CORE_TEXT)).toBe(CORE_TEXT)
    expect(refusalText(null, null, CORE_TEXT)).toBe(CORE_TEXT)
    // A name every object has is no code.
    expect(isRefusal("toString")).toBe(false)
    expect(refusalText("constructor", undefined, CORE_TEXT)).toBe(CORE_TEXT)
  })

  it("falls back to the core's sentence when the data lacks what its own names", () => {
    for (const data of [undefined, null, {}, { filename: 3 }])
      expect(refusalText("DATAPACK_NAME_TAKEN", data, CORE_TEXT)).toBe(
        CORE_TEXT
      )
    expect(refusalText("NO_DATAPACK_FOR_VERSION", {}, CORE_TEXT)).toBe(
      CORE_TEXT
    )
  })

  it("has nothing to say without a known code or a sentence from the core", () => {
    expect(refusalText(undefined, undefined, undefined)).toBeUndefined()
    expect(refusalText("A_CODE_FROM_A_NEWER_CORE", {}, null)).toBeUndefined()
    expect(refusalText("A_CODE_FROM_A_NEWER_CORE", {}, "")).toBeUndefined()
  })
})

// The core is the one that adds a refusal. Its sources are read here, so a
// code added there without a sentence here fails by name instead of reaching
// players as nothing, or in English.
describe("the codes the core refuses with", () => {
  const INSTANCE = "crates/carbon_app/src/managers/instance"
  const TABLE = "apps/desktop/packages/mainWindow/src/utils/refusalText.ts"
  const source = (path: string) =>
    readFileSync(new URL(`../../../../../../${path}`, import.meta.url), "utf8")
  /** The body of every `error_code` function of a Rust source. */
  const errorCodeBodies = (rust: string) =>
    [
      ...rust.matchAll(
        /fn error_code\(&self\) -> &'static str \{\n([\s\S]*?)\n {4}\}/g
      )
    ].map(([, body]) => body)
  const codesOf = (rust: string) =>
    errorCodeBodies(rust).flatMap((body) =>
      [...body.matchAll(/"([A-Z][A-Z0-9_]*)"/g)].map(([, code]) => code)
    )

  it("each have a sentence in the refusal table", () => {
    const path = `${INSTANCE}/world_refusal.rs`
    const [body, ...others] = errorCodeBodies(source(path))
    expect(others, `${path} has one error_code function`).toEqual([])
    const codes = codesOf(source(path))
    // Every arm of the match returns its code as one literal.
    expect(codes.length, `the codes read from ${path}`).toBe(
      body.match(/Self::/g)?.length
    )
    expect(codes.length).toBeGreaterThan(0)
    for (const code of codes)
      expect(
        isRefusal(code),
        `${path} refuses with ${code}, which has no sentence in ${TABLE}: add it to REFUSALS there, with its text in every language`
      ).toBe(true)
  })

  it("each have their sentence checked in every language", () => {
    const listed = new Set(REFUSALS.map(([code]) => code))
    for (const code of codesOf(source(`${INSTANCE}/world_refusal.rs`)))
      expect(
        listed.has(code),
        `${code} is missing from REFUSALS in this file, which is what every language is checked against`
      ).toBe(true)
  })

  // The two refusals of the feature the core codes outside world_refusal.rs,
  // with where the desktop handles each.
  it.each([
    {
      code: "NO_DATAPACK_FOR_VERSION",
      raisedIn: `${INSTANCE}/world_datapack_changes.rs`,
      handledIn: TABLE,
      sentence: true
    },
    {
      code: "ALREADY_IN_WORLD",
      raisedIn: `${INSTANCE}/datapacks.rs`,
      // Not a sentence: the Addons tab asks whether to replace the copy.
      handledIn:
        "apps/desktop/packages/mainWindow/src/pages/Library/Instance/Tabs/Addons/hooks/useDatapackWorlds.tsx",
      sentence: false
    }
  ])(
    "$code is still raised and handled where this says",
    ({ code, raisedIn, handledIn, sentence }) => {
      expect(
        codesOf(source(raisedIn)),
        `${code} is no longer a code of ${raisedIn}: say here where the core raises it now, or drop it with its handling in ${handledIn}`
      ).toContain(code)
      expect(
        source(handledIn),
        `${code} (${raisedIn}) is handled in ${handledIn}, which no longer names it`
      ).toContain(code)
      // A code with a sentence is told by the global handler; one handled
      // by a question of its own must not be told there as well.
      expect(
        isRefusal(code),
        sentence
          ? `${code} (${raisedIn}) has no sentence in ${TABLE}`
          : `${code} (${raisedIn}) is asked about in ${handledIn}; a sentence in ${TABLE} would tell it a second time`
      ).toBe(sentence)
    }
  )
})

describe("the world lists after a refusal", () => {
  const client = () => ({ invalidateQueries: vi.fn(async () => {}) })
  it.each([
    "WORLD_GONE",
    "WORLD_CHANGED",
    "WORLD_CHANGED_DURING_COPY",
    "DATAPACK_GONE",
    "DATAPACK_CHANGED"
  ])("%s has the instance's worlds read again", (code) => {
    const queries = client()
    refreshWorldsAfter(code, queries, 5)
    expect(queries.invalidateQueries.mock.calls).toEqual([
      [{ queryKey: ["instance.getWorlds", 5] }]
    ])
  })
  it("has every instance's worlds read again when the instance is not known", () => {
    const queries = client()
    refreshWorldsAfter("WORLD_GONE", queries)
    expect(queries.invalidateQueries.mock.calls).toEqual([
      [{ queryKey: ["instance.getWorlds"] }]
    ])
  })
  it.each([
    "INSTANCE_GAME_ACTIVE",
    "DATAPACK_NAME_TAKEN",
    "NO_DATAPACK_UPDATE",
    "INSTANCE_DATAPACK_GONE",
    "A_CODE_FROM_A_NEWER_CORE",
    undefined,
    null
  ])("%s leaves the lists as they are", (code) => {
    const queries = client()
    refreshWorldsAfter(code, queries, 5)
    expect(queries.invalidateQueries).not.toHaveBeenCalled()
  })
})
