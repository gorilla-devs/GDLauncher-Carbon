import { describe, expect, it } from "vitest"
import {
  countMatchedMarkers,
  findFatalSignature,
  LAUNCH_MARKERS,
  parseLogMessages
} from "./gameLog.js"

/** One log4j event in the exact shape the launcher writes — see
 *  `format_message_as_log4j_event` in the Rust core, and the captured probe
 *  log this fixture was copied from. */
function event(message: string, level = "INFO"): string {
  return [
    `<log4j:Event logger="GDLAUNCHER" timestamp="1785492632751" level="${level}" thread="N/A">`,
    `\t<log4j:Message><![CDATA[${message}]]></log4j:Message>`,
    `</log4j:Event>`
  ].join("\n")
}

describe("parseLogMessages", () => {
  it("extracts each event's message payload", () => {
    const raw = [
      event("Setting user: e2e_2be5124c"),
      event("Sound engine started")
    ].join("\n")

    expect(parseLogMessages(raw)).toEqual([
      "Setting user: e2e_2be5124c",
      "Sound engine started"
    ])
  })

  it("keeps multi-line payloads intact", () => {
    // The launcher writes pretty-printed Rust structs into single events —
    // "Using Java: JavaComponent { ... }" spans a dozen lines in a real log.
    const raw = event("Using Java: JavaComponent {\n    major: 17,\n}")

    const messages = parseLogMessages(raw)
    expect(messages).toHaveLength(1)
    expect(messages[0]).toContain("major: 17")
  })

  it("returns nothing for output that is not log4j events", () => {
    expect(parseLogMessages("plain stdout, no events here")).toEqual([])
  })
})

describe("countMatchedMarkers", () => {
  it("counts distinct markers, not total matches", () => {
    // Eleven atlases are created in a real startup; that is still one marker
    // satisfied, not eleven. Counting matches instead of markers would let a
    // single chatty subsystem reach quorum on its own.
    const messages = [
      "Created: 1024x512x4 minecraft:textures/atlas/blocks.png-atlas",
      "Created: 256x256x4 minecraft:textures/atlas/signs.png-atlas",
      "Created: 512x512x4 minecraft:textures/atlas/beds.png-atlas"
    ]

    expect(countMatchedMarkers(messages, LAUNCH_MARKERS)).toBe(1)
  })

  it("counts each different marker that matched", () => {
    const messages = [
      "Setting user: e2e_2be5124c",
      "Backend library: LWJGL version 3.3.1 build 7",
      "Sound engine started"
    ]

    expect(countMatchedMarkers(messages, LAUNCH_MARKERS)).toBe(3)
  })

  it("counts nothing when the log says nothing recognisable", () => {
    expect(
      countMatchedMarkers(["something else entirely"], LAUNCH_MARKERS)
    ).toBe(0)
  })
})

describe("findFatalSignature", () => {
  it("finds an uncaught main-thread exception", () => {
    const messages = [
      "Setting user: e2e",
      'Exception in thread "main" java.lang.NoClassDefFoundError: net/minecraft/client/main/Main'
    ]

    expect(findFatalSignature(messages)).toContain("Exception in thread")
  })

  it("finds a JVM-level fatal error", () => {
    const messages = [
      "# A fatal error has been detected by the Java Runtime Environment:"
    ]

    expect(findFatalSignature(messages)).toContain(
      "fatal error has been detected"
    )
  })

  it("does not treat the mock account's expected auth failures as fatal", () => {
    // Both of these appear in every launch under the e2e mock account, which
    // holds a token real session servers reject. Treating them as failures
    // would make this assertion permanently red rather than meaningful.
    const messages = [
      "Failed to verify authentication",
      "Failed to retrieve profile key pair",
      "Could not authorize you against Realms server: java.lang.RuntimeException"
    ]

    expect(findFatalSignature(messages)).toBeUndefined()
  })
})
