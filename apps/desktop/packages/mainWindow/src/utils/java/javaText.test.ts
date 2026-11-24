import { describe, expect, test, vi } from "vitest"
import javaEnglish from "../../../../../../../packages/i18n/locale/english/java.json"
import {
  firstNames,
  formatMemory,
  formatSize,
  javaNeedText,
  knownIssueNames,
  labelHints,
  memoryMarks,
  middleTruncate,
  securityFlawText
} from "./javaText"

// Renders keys with the shipped English copy, so assertions read as the UI.
const t = (key: string, options: Record<string, unknown> = {}) => {
  const template = (javaEnglish as Record<string, string>)[
    key.replace(/^java:/, "")
  ]
  if (template === undefined) throw new Error(`missing string ${key}`)
  return template.replace(/{{(\w+)}}/g, (_, name: string) =>
    String(options[name])
  )
}

describe("knownIssueNames", () => {
  test("lists each issue once, in the app's language", () => {
    const names = ["Java4Shell", "Log4Shell", "Java4Shell"]
    expect(knownIssueNames(names, "english")).toBe("Java4Shell and Log4Shell")
    expect(knownIssueNames(names, "italian")).toBe("Java4Shell e Log4Shell")
    expect(knownIssueNames(["Java4Shell"], "russian")).toBe("Java4Shell")
  })
})

describe("securityFlawText", () => {
  test("says it's a security flaw before naming it", () => {
    expect(securityFlawText(["Java4Shell", "Log4Shell"], "english", t)).toBe(
      "Security flaw: Java4Shell and Log4Shell"
    )
  })

  test("asks for the names unescaped, as it's text, not HTML", () => {
    // It ends up in a tooltip and in a radio's description, where an
    // escaped "'" would read as "&#39;".
    const spy = vi.fn(t)
    securityFlawText(["Xaero's flaw"], "english", spy)
    expect(spy).toHaveBeenCalledWith(
      "java:_trn_security_flaw",
      expect.objectContaining({ interpolation: { escapeValue: false } })
    )
  })
})

describe("formatSize", () => {
  // Each unit renders as its key, so the test sees which one was picked.
  const unit = (key: string) => key.replace("java:_trn_unit_", "").toUpperCase()

  test("formats in the app's language with a translated unit", () => {
    const mb = 198_600_000
    expect(formatSize(mb, "english", unit)).toBe("189.4\u00a0MB")
    expect(formatSize(mb, "russian", unit)).toBe("189,4\u00a0MB")
    expect(formatSize(512, "english", unit)).toBe("512\u00a0B")
    expect(formatSize(1536, "french", unit)).toBe("1,5\u00a0KB")
    expect(formatSize(3 * 1024 ** 3, "english", unit)).toBe("3.00\u00a0GB")
    expect(formatSize(0, "english", unit)).toBe("0\u00a0B")
  })
})

describe("formatMemory", () => {
  const unit = (key: string) => key.replace("java:_trn_unit_", "").toUpperCase()

  test("reads whole gigabytes without decimals, in the app's language", () => {
    expect(formatMemory(4096, "english", unit)).toBe("4\u00a0GB")
    expect(formatMemory(1536, "russian", unit)).toBe("1,5\u00a0GB")
    expect(formatMemory(512, "english", unit)).toBe("512\u00a0MB")
  })
})

describe("memoryMarks", () => {
  const gb = (mb: number) => `${mb / 1024} GB`

  test("labels each doubling, hiding a label too close to the one before", () => {
    // 1 GB and 2 GB sit 1.6% apart on a 64 GB slider: 2 GB keeps its tick
    // but loses its label.
    expect(memoryMarks(1024, 65536, gb)).toEqual({
      1024: "1 GB",
      2048: "",
      4096: "4 GB",
      8192: "8 GB",
      16384: "16 GB",
      32768: "32 GB",
      65536: "64 GB"
    })
    // On 16 GB everything fits.
    expect(memoryMarks(1024, 16384, gb)).toEqual({
      1024: "1 GB",
      2048: "2 GB",
      4096: "4 GB",
      8192: "8 GB",
      16384: "16 GB"
    })
  })
})

describe("labelHints", () => {
  const java = (
    id: string,
    path: string,
    source: "managed" | "local" | "custom" = "custom",
    vendorName = "Azul Zulu",
    version = "17.0.16"
  ) => ({ id, path, source, vendorName, version })

  test("tells apart Javas that share a name, and leaves the rest alone", () => {
    const hints = labelHints([
      java("a", "/home/steve/user-java/zulu17/bin/java"),
      java("b", "/home/steve/user-java/zulu17-copy/bin/java"),
      java(
        "c",
        "/gdl/managed_javas/zulu21.40.17-ca-jre21.0.10-linux_x64/bin/java",
        "managed",
        "Azul Zulu",
        "21.0.10"
      ),
      java(
        "d",
        "/gdl/managed_javas/zulu21.38.21-ca-jre21.0.10-linux_x64/bin/java",
        "managed",
        "Azul Zulu",
        "21.0.10"
      ),
      java("e", "/opt/graalvm-21/bin/java", "local", "GraalVM", "21.0.2")
    ])

    expect(hints).toEqual(
      new Map([
        ["a", "zulu17"],
        ["b", "zulu17-copy"],
        // GDLauncher's own: Azul's build name, not the whole folder.
        ["c", "zulu21.40.17"],
        ["d", "zulu21.38.21"]
      ])
    )
  })

  test("never names a Mojang folder, which holds a codename", () => {
    const mojang = (id: string) =>
      java(
        id,
        `/gdl/managed_javas/mojang-java-runtime-delta-21.0.7-${id}/bin/java`,
        "managed",
        "Mojang",
        "21.0.7"
      )
    expect(labelHints([mojang("a"), mojang("b")])).toEqual(new Map())
  })
})

describe("middleTruncate", () => {
  test("leaves a path that fits unchanged", () => {
    expect(middleTruncate("/opt/graalvm-21/bin/java", 60)).toBe(
      "/opt/graalvm-21/bin/java"
    )
  })

  test("keeps the first folder and as much of the tail as fits", () => {
    expect(
      middleTruncate("/usr/lib/jvm/java-21-openjdk-amd64/bin/java", 30)
    ).toBe("/usr/…/bin/java")
  })

  test("keeps the drive and first folder of a Windows path", () => {
    const path =
      "C:\\Users\\steve\\AppData\\Roaming\\gdlauncher_carbon\\data\\jdk-21\\bin\\java.exe"
    const short = middleTruncate(path, 40)
    expect(short).toBe("C:\\Users\\…\\data\\jdk-21\\bin\\java.exe")
    expect(short.length).toBeLessThanOrEqual(40)
  })

  test("always keeps the file name, even when it alone is too long", () => {
    expect(
      middleTruncate("/home/steve/java/a-very-long-binary-name-for-java", 20)
    ).toBe("/home/…/a-very-long-binary-name-for-java")
  })
})

describe("firstNames", () => {
  test("keeps the first 8 and counts the rest", () => {
    const names = Array.from({ length: 11 }, (_, i) => `Instance ${i}`)

    expect(firstNames(names)).toEqual({ shown: names.slice(0, 8), hidden: 3 })
    expect(firstNames(names.slice(0, 8))).toEqual({
      shown: names.slice(0, 8),
      hidden: 0
    })
    expect(firstNames([])).toEqual({ shown: [], hidden: 0 })
  })
})

describe("javaNeedText", () => {
  test("says which Java the Minecraft version needs", () => {
    expect(javaNeedText("1.21.4", 21, null, t)).toBe(
      "Minecraft 1.21.4 needs Java 21"
    )
    // Only Forge on 1.16.5 caps the build; vanilla 1.16.5 doesn't need it.
    expect(javaNeedText("1.16.5", 8, "8.0.312", t)).toBe(
      "Forge 1.16.5 needs Java 8 older than 8.0.312"
    )
    // After a wrong-Java crash raised what the instance needs.
    expect(javaNeedText("1.20.1", 21, null, t, 21)).toBe(
      "This instance needs Java 21 or later"
    )
  })
})
