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
  ["INSTANCE_DATAPACK_CHANGED"]
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
