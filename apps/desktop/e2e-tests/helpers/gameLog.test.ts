import { describe, expect, it } from "vitest"
import {
  countMatchedMarkers,
  findStopOpeningWindow,
  findFatalSignature,
  findGraphicsUnavailable,
  judgeGraphicsUnavailable,
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

describe("findGraphicsUnavailable", () => {
  // Forge's early-window error from develop's windows-2022 run 32960118348,
  // as one multi-line log4j event.
  const forgeEarlyWindow = [
    "ERROR DISPLAY",
    "Failed to initialize graphics window with current settings.",
    "",
    "Failure details:",
    "Failed to find a valid GLFW profile.",
    "We tried 4.6, 4.5, 4.4, 4.3, 4.2, 4.1, 4.0, 3.3, 3.2 but none of them worked.",
    "Trying 4.6: GLFW error: [0x10006]WGL: The driver does not appear to support OpenGL",
    "Trying 3.2: GLFW error: [0x10006]WGL: The driver does not appear to support OpenGL"
  ].join("\n")

  it("names the driver line of a log that has no OpenGL driver", () => {
    expect(
      findGraphicsUnavailable(
        parseLogMessages(
          [event("Setting user: e2e"), event(forgeEarlyWindow, "ERROR")].join(
            "\n"
          )
        )
      )
    ).toBe(
      "Trying 4.6: GLFW error: [0x10006]WGL: The driver does not appear to support OpenGL"
    )
  })

  it("finds the macOS runner's missing pixel format (run 41466362930)", () => {
    const line =
      "Trying 4.6: GLFW error: [0x10009]NSGL: Failed to find a suitable pixel format"
    expect(
      findGraphicsUnavailable(
        parseLogMessages(
          event(
            [
              "ERROR DISPLAY",
              "Failed to find a valid GLFW profile.",
              line
            ].join("\n"),
            "ERROR"
          )
        )
      )
    ).toBe(line)
  })

  it("finds the JVM crash banner naming the Windows OpenGL driver (run 41774657124)", () => {
    // The JVM prints its fatal-error banner raw, between two log4j events.
    const raw = [
      event("Backend library: LWJGL version 3.3.1 SNAPSHOT"),
      "#",
      "# A fatal error has been detected by the Java Runtime Environment:",
      "#",
      "#  EXCEPTION_ACCESS_VIOLATION (0xc0000005) at pc=0x00007ffb944294da, pid=6524, tid=4364",
      "#",
      "# Problematic frame:",
      "# C  [opengl32.dll+0x294da]",
      "#",
      event("exit code: 1")
    ].join("\r\n")
    expect(findGraphicsUnavailable([...parseLogMessages(raw), raw])).toBe(
      "# C  [opengl32.dll+0x294da]"
    )
  })

  it("finds nothing in a log that only failed to authenticate", () => {
    expect(
      findGraphicsUnavailable([
        "Setting user: e2e",
        "Failed to verify authentication",
        "Backend library: LWJGL version 3.3.1 build 7",
        "Sound engine started"
      ])
    ).toBeUndefined()
  })
})

describe("findStopOpeningWindow", () => {
  // The macos-15 game the system killed in run 41774657124, in log order.
  const killedOpeningWindow = [
    "Setting user: e2e",
    "LWJGL Meta org.lwjgl3 - 3.3.1",
    "Backend library: LWJGL version 3.3.1 SNAPSHOT",
    "Update available for 'fabric-api@0.92.6+1.20.1', (-> 0.92.12+1.20.1)",
    "signal: 9 (SIGKILL)"
  ]

  it("names the window-system line where CI declares no OpenGL driver", () => {
    expect(findStopOpeningWindow(killedOpeningWindow, true)).toBe(
      "Backend library: LWJGL version 3.3.1 SNAPSHOT"
    )
  })

  it("trusts nothing where no OpenGL driver is declared missing", () => {
    expect(findStopOpeningWindow(killedOpeningWindow, false)).toBeUndefined()
  })

  it("names it for a game that hung there instead (run 41799780827)", () => {
    expect(
      findStopOpeningWindow(
        [
          "Setting user: e2e",
          "Backend library: LWJGL version 3.3.1 SNAPSHOT",
          "Update available for 'c2me@0.2.0+alpha.11.16', (-> 0.2.0+alpha.11.18+1.20.1)"
        ],
        true
      )
    ).toBe("Backend library: LWJGL version 3.3.1 SNAPSHOT")
  })

  it("finds nothing once the window opened", () => {
    expect(
      findStopOpeningWindow(
        [...killedOpeningWindow, "Reloading ResourceManager: vanilla, fabric"],
        true
      )
    ).toBeUndefined()
  })

  it("finds nothing for a game that died before its window system", () => {
    expect(
      findStopOpeningWindow(
        ["Setting user: e2e", 'Exception in thread "main" java.lang.Error'],
        true
      )
    ).toBeUndefined()
  })
})

describe("judgeGraphicsUnavailable", () => {
  const wglLine =
    "Trying 3.2: GLFW error: [0x10006]WGL: The driver does not appear to support OpenGL"
  const noDriver = [
    event("Setting user: e2e"),
    event(
      ["ERROR DISPLAY", "Failed to find a valid GLFW profile.", wglLine].join(
        "\n"
      ),
      "ERROR"
    )
  ].join("\n")
  // The macos-15 game the system killed in run 41774657124.
  const stoppedOpeningWindow = [
    event("Setting user: e2e"),
    event("Backend library: LWJGL version 3.3.1 SNAPSHOT"),
    event("signal: 9 (SIGKILL)")
  ].join("\n")

  it("skips a log naming no OpenGL driver where CI declares nothing", () => {
    const verdict = judgeGraphicsUnavailable(noDriver, {
      noOpenGl: false,
      openGlRequired: false
    })
    expect(verdict?.outcome).toBe("skip")
    expect(verdict?.reason).toContain(wglLine)
  })

  it("fails it instead where CI declares games have OpenGL", () => {
    // A broken Mesa install on the Windows runner must not quietly turn the
    // real game launches back into skips.
    const verdict = judgeGraphicsUnavailable(noDriver, {
      noOpenGl: false,
      openGlRequired: true
    })
    expect(verdict?.outcome).toBe("fail")
    expect(verdict?.reason).toContain("GDL_E2E_REQUIRE_OPENGL")
    expect(verdict?.reason).toContain(wglLine)
  })

  it("fails on the JVM crash banner in the Windows OpenGL driver too", () => {
    // Printed raw, outside any log4j event (run 41774657124).
    const raw = [
      event("Backend library: LWJGL version 3.3.1 SNAPSHOT"),
      "# Problematic frame:",
      "# C  [opengl32.dll+0x294da]",
      event("exit code: 1")
    ].join("\r\n")
    const verdict = judgeGraphicsUnavailable(raw, {
      noOpenGl: false,
      openGlRequired: true
    })
    expect(verdict?.outcome).toBe("fail")
    expect(verdict?.reason).toContain("# C  [opengl32.dll+0x294da]")
  })

  it("skips a game that stopped opening its window where CI declares no OpenGL", () => {
    const verdict = judgeGraphicsUnavailable(stoppedOpeningWindow, {
      noOpenGl: true,
      openGlRequired: false
    })
    expect(verdict?.outcome).toBe("skip")
    expect(verdict?.reason).toContain(
      "Backend library: LWJGL version 3.3.1 SNAPSHOT"
    )
  })

  it("leaves a game that stopped opening its window to the caller where OpenGL is required", () => {
    // Only GDL_E2E_NO_OPENGL makes that stop a skip; on a machine with a
    // driver it is a real failure the caller's assertions report.
    expect(
      judgeGraphicsUnavailable(stoppedOpeningWindow, {
        noOpenGl: false,
        openGlRequired: true
      })
    ).toBeUndefined()
  })

  it("leaves a log without either to the caller", () => {
    const started = [
      event("Setting user: e2e"),
      event("Backend library: LWJGL version 3.3.1 build 7"),
      event("Sound engine started")
    ].join("\n")
    for (const noOpenGl of [false, true]) {
      for (const openGlRequired of [false, true]) {
        expect(
          judgeGraphicsUnavailable(started, { noOpenGl, openGlRequired })
        ).toBeUndefined()
      }
    }
  })
})
