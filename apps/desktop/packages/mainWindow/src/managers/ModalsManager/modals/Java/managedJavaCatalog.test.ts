import { test, expect } from "vitest"
import {
  buildHints,
  catalogForMajors,
  compareJavaVersions,
  defaultBuild,
  defaultMajor,
  filterBuilds,
  formatReleaseDate,
  isBuildInstalled,
  majorLabel,
  progressLabel,
  progressView,
  vendorLabel,
  visibleMajors
} from "./managedJavaCatalog"

const b = (version: string, name = `zulu-${version}`) => ({
  id: version,
  name,
  major: Number(version.split(".")[0]),
  version,
  releaseDate: null,
  sizeBytes: null,
  knownIssues: []
})
const cat = (...ms: number[]) => ({
  majors: ms.map((major) => ({
    major,
    lts: [8, 11, 17, 21, 25].includes(major),
    builds: []
  }))
})
const mc = [
  {
    major: 25,
    fromVersion: "26.1",
    toVersion: "26.1",
    newest: true,
    oldest: false
  },
  {
    major: 21,
    fromVersion: "1.20.5",
    toVersion: "1.21.11",
    newest: false,
    oldest: false
  },
  {
    major: 16,
    fromVersion: "1.17",
    toVersion: "1.17",
    newest: false,
    oldest: false
  },
  {
    major: 8,
    fromVersion: "1.0",
    toVersion: "1.16.5",
    newest: false,
    oldest: true
  }
]
const t = (k: string, o: unknown) => `${k}|${JSON.stringify(o)}`

test("defaultMajor prefers the requested major, then the newest Minecraft major", () => {
  expect(defaultMajor(cat(27, 25, 21), mc, 21)).toBe(21)
  expect(defaultMajor(cat(27, 25, 21), mc)).toBe(25)
  expect(defaultMajor(cat(27, 25, 21), mc, 99)).toBe(25)
  expect(defaultMajor(cat(27), mc)).toBe(27)
})

test("visibleMajors hides non-Minecraft majors and tolerates a missing 16", () => {
  expect(visibleMajors(cat(27, 25, 22, 21, 8), mc, false)).toEqual([25, 21, 8])
  expect(visibleMajors(cat(27, 25, 22, 21, 8), mc, true)).toEqual([
    27, 25, 22, 21, 8
  ])
})

test("without the catalog, the majors come from Minecraft's data", () => {
  // The download server is unreachable: the select still lists the Javas
  // Minecraft uses.
  expect(visibleMajors(undefined, mc, false)).toEqual([25, 21, 16, 8])
  expect(visibleMajors(undefined, mc, true)).toEqual([25, 21, 16, 8])
  expect(defaultMajor(undefined, mc, 21)).toBe(21)
  expect(defaultMajor(undefined, mc, 99)).toBe(25)
  expect(defaultMajor(undefined, [])).toBeUndefined()
})

test("catalogForMajors keeps Minecraft's majors while a failed catalog retries", () => {
  const c = cat(25, 21)
  expect(catalogForMajors({ data: c, failed: false })).toBe(c)
  // A catalog that arrived wins over an earlier failure.
  expect(catalogForMajors({ data: c, failed: true })).toBe(c)
  // Loading: nothing to offer yet.
  expect(catalogForMajors({ data: undefined, failed: false })).toBeNull()
  // Failed, whether or not "Try again" is refetching: Minecraft's majors.
  expect(catalogForMajors({ data: undefined, failed: true })).toBeUndefined()
})

test("buildHints names Azul's build only where versions clash", () => {
  const builds = [
    b("21.0.13.1", "zulu21.46.19-ca-jre21.0.13.1-linux_x64.zip"),
    b("21.0.10", "zulu21.40.17-ca-jre21.0.10-linux_x64.zip"),
    { ...b("21.0.10", "zulu21.38.21-ca-jre21.0.10-linux_x64.zip"), id: "x" }
  ]
  expect(buildHints(builds, "azul")).toEqual(
    new Map([
      ["21.0.10", "zulu21.40.17"],
      ["x", "zulu21.38.21"]
    ])
  )
  // Mojang's build names are codenames.
  expect(buildHints(builds, "mojang")).toEqual(new Map())
})

test("majorLabel uses ranges, newer/older ends, and LTS only for extra majors", () => {
  const withMajor = (major: number, range: string) =>
    `java:_trn_major_minecraft|${JSON.stringify({ major, range })}`

  expect(majorLabel(21, true, mc, t)).toBe(
    withMajor(21, 'java:_trn_minecraft_range|{"from":"1.20.5","to":"1.21.11"}')
  )
  expect(majorLabel(25, true, mc, t)).toBe(
    withMajor(25, 'java:_trn_minecraft_range_newer|{"from":"26.1"}')
  )
  expect(majorLabel(8, true, mc, t)).toBe(
    withMajor(8, 'java:_trn_minecraft_range_older|{"to":"1.16.5"}')
  )
  expect(majorLabel(16, true, mc, t)).toBe(withMajor(16, "1.17"))
  expect(majorLabel(11, true, mc, t)).toBe('java:_trn_major_lts|{"major":11}')
  expect(majorLabel(22, false, mc, t)).toBe('java:_trn_major|{"major":22}')
})

test("majorLabel falls back to the plain major without version data", () => {
  const offline = mc.map((m) => ({ ...m, fromVersion: "", toVersion: "" }))
  expect(majorLabel(21, true, offline, t)).toBe(
    'java:_trn_major_lts|{"major":21}'
  )
  expect(majorLabel(25, false, offline, t)).toBe('java:_trn_major|{"major":25}')
})

test("filterBuilds matches version and name, case-insensitive, keeps order", () => {
  const list = [b("21.0.13.1"), b("21.0.11", "ZULU21.50.19")]
  expect(filterBuilds(list, "13.1").map((x) => x.version)).toEqual([
    "21.0.13.1"
  ])
  expect(filterBuilds(list, "zulu21.50").map((x) => x.version)).toEqual([
    "21.0.11"
  ])
  expect(filterBuilds(list, "")).toEqual(list)
})

test("progressView parses download and extract tuples", () => {
  expect(progressView({ downloading: ["50", "200"] })).toEqual({
    kind: "download",
    done: 50,
    total: 200,
    fraction: 0.25
  })
  expect(progressView({ extracting: ["3", "0"] })).toEqual({
    kind: "extract",
    done: 3,
    total: 0,
    fraction: undefined
  })
  expect(progressView("idle").kind).toBe("idle")
})

test("isBuildInstalled ignores trailing zeros and non-managed javas", () => {
  const javas = [
    {
      type: "managed",
      version: "17.0.21.1",
      path: "/x/managed_javas/zulu17.70.201-ca-jre17.0.21.1-linux_x64/bin/java"
    },
    {
      type: "local",
      version: "21.0.13.1",
      path: "/x/managed_javas/zulu21-local/bin/java"
    }
  ] as any
  expect(isBuildInstalled(b("17.0.21.1.0"), javas, "azul")).toBe(true)
  expect(isBuildInstalled(b("21.0.13.1"), javas, "azul")).toBe(false)
})

test("isBuildInstalled tells Azul and Mojang builds of one version apart", () => {
  const javas = [
    {
      type: "managed",
      version: "17.0.15",
      path: "C:\\x\\managed_javas\\mojang-java-runtime-gamma-17.0.15\\bin\\java.exe"
    }
  ] as any
  expect(
    isBuildInstalled(b("17.0.15", "java-runtime-gamma"), javas, "mojang")
  ).toBe(true)
  expect(isBuildInstalled(b("17.0.15"), javas, "azul")).toBe(false)
})

test("isBuildInstalled matches a Mojang build by install folder", () => {
  const alpha = { ...b("16.0.1.9.1", "java-runtime-alpha"), major: 16 }
  const javas = [
    {
      type: "managed",
      // The binary reports a shorter version than the one Mojang lists.
      version: "16.0.1",
      path: "/home/me/managed_javas/mojang-java-runtime-alpha-16.0.1.9.1/bin/java"
    }
  ] as any
  expect(isBuildInstalled(alpha, javas, "mojang")).toBe(true)
  expect(isBuildInstalled(alpha, [], "mojang")).toBe(false)
})

test("isBuildInstalled reads the distribution from the folder under managed_javas", () => {
  const javas = [
    {
      type: "managed",
      version: "17.0.15",
      path: "/home/zuluaga/managed_javas/mojang-java-runtime-gamma-17.0.15/bin/java"
    }
  ] as any
  expect(isBuildInstalled(b("17.0.15"), javas, "azul")).toBe(false)
})

test("vendorLabel names each distribution", () => {
  expect(vendorLabel("mojang")).toBe("Mojang")
  expect(vendorLabel("azul")).toBe("Azul Zulu")
})

test("progressLabel formats bytes and file counts, and nothing without a total", () => {
  const bytes = (n: number) => `${n}B`
  expect(
    progressLabel(progressView({ downloading: ["50", "200"] }), t, bytes)
  ).toBe('java:_trn_install_downloading|{"done":"50B","total":"200B"}')
  expect(
    progressLabel(progressView({ extracting: ["3", "9"] }), t, bytes)
  ).toBe('java:_trn_install_extracting|{"done":3,"total":9}')
  expect(
    progressLabel(progressView({ downloading: ["50", "0"] }), t, bytes)
  ).toBeUndefined()
  expect(progressLabel(progressView("idle"), t, bytes)).toBeUndefined()
})

test("formatReleaseDate maps app language names to locales", () => {
  const iso = "2026-11-17T12:00:00Z"
  expect(formatReleaseDate(iso, "english")).toBe(
    new Intl.DateTimeFormat("en", { dateStyle: "medium" }).format(new Date(iso))
  )
  // "simplified_chinese" is not a valid locale tag and would throw.
  expect(formatReleaseDate(iso, "simplified_chinese")).toBe(
    new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" }).format(
      new Date(iso)
    )
  )
  expect(formatReleaseDate(null, "english")).toBeUndefined()
  expect(formatReleaseDate("not a date", "english")).toBeUndefined()
})

test("compareJavaVersions compares numerically, with the legacy Java 8 form and suffixes", () => {
  const sorted = [
    "17.0.21.1",
    "17.0.21.0.3",
    "1.8.0_492",
    "17.0.9",
    "21.0.13.1+11",
    "8.0.312"
  ].sort(compareJavaVersions)

  expect(sorted).toEqual([
    "8.0.312",
    "1.8.0_492",
    "17.0.9",
    "17.0.21.0.3",
    "17.0.21.1",
    "21.0.13.1+11"
  ])
  expect(compareJavaVersions("1.8.0_492", "8.0.492")).toBe(0)
})

test("defaultBuild is the newest build, or the newest below a cap", () => {
  const builds = [b("8.0.492"), b("8.0.482"), b("8.0.302"), b("8.0.292")]

  expect(defaultBuild(builds, undefined)?.version).toBe("8.0.492")
  // Forge 1.16.5 needs Java 8 below 8.0.312.
  expect(defaultBuild(builds, "8.0.312")?.version).toBe("8.0.302")
  // With nothing below the cap, still the newest.
  expect(defaultBuild([b("8.0.492")], "8.0.312")?.version).toBe("8.0.492")
  expect(defaultBuild([], "8.0.312")).toBeUndefined()
})
