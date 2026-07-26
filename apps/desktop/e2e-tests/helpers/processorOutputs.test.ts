import path from "node:path"
import { describe, expect, it } from "vitest"
import {
  requiredLibraryPaths,
  type Processor,
  type RequiredLibrary,
  type SidedDataEntry
} from "./processorOutputs.js"

// Copied verbatim from `processor_outputs.rs`'s own test fixtures
// (`FORGE_1_20_1_PROCESSORS`/`_DATA`, `NEO_26_PROCESSORS`/`_DATA`) rather
// than re-derived, so a divergence between this TS port and the Rust source
// it mirrors shows up as a failing assertion here instead of surfacing for
// the first time against a real, seeded-random loader build in
// `loaderInstall.spec.ts`. See that Rust module's own doc comments for what
// each fixture is trimmed from and what it covers (a server-only processor
// that must be excluded, declared outputs with SHAs, a sides-less processor
// without a SHA, and the K/K_SHA data-pairing fallback).
const FORGE_1_20_1_PROCESSORS = `[
    {"jar":"net.minecraftforge:installertools:1.3.0","classpath":[],
     "args":["--task","BUNDLER_EXTRACT","--input","{MINECRAFT_JAR}","--output","{MC_UNPACKED}","--jar-only"],
     "sides":["server"]},
    {"jar":"net.minecraftforge:jarsplitter:1.1.4","classpath":[],
     "args":["--input","{MINECRAFT_JAR}","--slim","{MC_SLIM}","--extra","{MC_EXTRA}","--srg","{MERGED_MAPPINGS}"],
     "outputs":{"{MC_SLIM}":"{MC_SLIM_SHA}","{MC_EXTRA}":"{MC_EXTRA_SHA}"},
     "sides":["client"]},
    {"jar":"net.minecraftforge:ForgeAutoRenamingTool:0.1.22:all","classpath":[],
     "args":["--input","{MC_SLIM}","--output","{MC_SRG}","--names","{MERGED_MAPPINGS}","--ann-fix","--ids-fix","--src-fix","--record-fix"]},
    {"jar":"net.minecraftforge:binarypatcher:1.1.1","classpath":[],
     "args":["--clean","{MC_SRG}","--output","{PATCHED}","--apply","{BINPATCH}"]}
]`

const FORGE_1_20_1_DATA = `{
    "MC_SLIM":{"client":"[net.minecraft:client:1.20.1-20230612.114412:slim]","server":"[net.minecraft:server:1.20.1-20230612.114412:slim]"},
    "MC_SLIM_SHA":{"client":"'de86b035d2da0f78940796bb95c39a932ed84834'","server":"'9e06bdd77ca6d95b2cced0bf372245f753eeb16a'"},
    "MC_EXTRA":{"client":"[net.minecraft:client:1.20.1-20230612.114412:extra]","server":"[net.minecraft:server:1.20.1-20230612.114412:extra]"},
    "MC_EXTRA_SHA":{"client":"'8c5a95cbce940cfdb304376ae9fea47968d02587'","server":"'13522e3278befd103064d91a199451df4cd2633f'"},
    "MC_SRG":{"client":"[net.minecraft:client:1.20.1-20230612.114412:srg]","server":"[net.minecraft:server:1.20.1-20230612.114412:srg]"},
    "PATCHED":{"client":"[net.minecraftforge:forge:1.20.1-47.2.0:client]","server":"[net.minecraftforge:forge:1.20.1-47.2.0:server]"},
    "PATCHED_SHA":{"client":"'3e175b011146785588f1649a20d1834d10282a7c'","server":"'597ed4e82a3e309572f7ceafa2408571ba7e6c43'"},
    "BINPATCH":{"client":"[net.minecraftforge:forge:1.20.1-forge-47.2.0:client@lzma]","server":"[net.minecraftforge:forge:1.20.1-forge-47.2.0:server@lzma]"},
    "MC_UNPACKED":{"client":"[net.minecraft:client:1.20.1-20230612.114412:unpacked]","server":"[net.minecraft:server:1.20.1-20230612.114412:unpacked]"},
    "MERGED_MAPPINGS":{"client":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]","server":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]"}
}`

const NEO_26_PROCESSORS = `[
    {"jar":"net.neoforged.installertools:installertools:4.0.12:fatjar","classpath":[],
     "args":["--task","EXTRACT_FILES","--archive","{INSTALLER}","--from","data/run.sh","--to","{ROOT}/run.sh"],
     "sides":["server"]},
    {"jar":"net.neoforged.installertools:installertools:4.0.12:fatjar","classpath":[],
     "args":["--task","PROCESS_MINECRAFT_JAR","--no-mod-manifest","--input","{MINECRAFT_JAR}","--output","{PATCHED}","--extract-libraries-to","{ROOT}/libraries/","--apply-patches","{BINPATCH}"]}
]`

const NEO_26_DATA = `{
    "PATCHED":{"client":"[net.neoforged:minecraft-client-patched:26.2.0.23-beta]","server":"[net.neoforged:minecraft-server-patched:26.2.0.23-beta]"},
    "BINPATCH":{"client":"[net.minecraftforge:forge:neoforge-26.2.0.23-beta:client@lzma]","server":"[net.minecraftforge:forge:neoforge-26.2.0.23-beta:client@lzma]"}
}`

function parse(
  processors: string,
  data: string
): [Processor[], Record<string, SidedDataEntry>] {
  return [JSON.parse(processors), JSON.parse(data)]
}

function find(required: RequiredLibrary[], suffix: string): RequiredLibrary {
  const match = required.find((f) =>
    f.relativePath.endsWith(suffix.replace(/\//g, path.sep))
  )
  if (!match) {
    throw new Error(
      `no required file ending in ${suffix}: ${JSON.stringify(required)}`
    )
  }
  return match
}

describe("requiredLibraryPaths", () => {
  it("forge 1.20.1: required set is exactly the six client files", () => {
    // Mirrors processor_outputs.rs's forge_1_20_1_required_set_is_exactly_the_six_client_files.
    const [procs, data] = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA)
    const required = requiredLibraryPaths(procs, data)

    expect(required).toHaveLength(6)
    find(
      required,
      "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-slim.jar"
    )
    find(
      required,
      "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-extra.jar"
    )
    find(
      required,
      "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-srg.jar"
    )
    find(
      required,
      "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412-mappings-merged.txt"
    )
    find(
      required,
      "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-client.jar"
    )
    find(
      required,
      "net/minecraftforge/forge/1.20.1-forge-47.2.0/forge-1.20.1-forge-47.2.0-client.lzma"
    )
  })

  it("excludes server-only data refs", () => {
    // Mirrors processor_outputs.rs's server_only_data_refs_are_excluded:
    // MC_UNPACKED is only referenced by a server-sided (BUNDLER_EXTRACT)
    // processor.
    const [procs, data] = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA)
    const required = requiredLibraryPaths(procs, data)
    expect(required.some((f) => f.relativePath.includes("unpacked"))).toBe(
      false
    )
  })

  it("extracts SHAs from declared outputs and the K/K_SHA data pairing", () => {
    // Mirrors processor_outputs.rs's
    // sha_extraction_from_declared_outputs_and_key_sha_pairing.
    const [procs, data] = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA)
    const required = requiredLibraryPaths(procs, data)

    const slim = find(required, "client-1.20.1-20230612.114412-slim.jar")
    expect(slim.expectedSha1).toBe("de86b035d2da0f78940796bb95c39a932ed84834")

    const patched = find(required, "forge-1.20.1-47.2.0-client.jar")
    expect(patched.expectedSha1).toBe(
      "3e175b011146785588f1649a20d1834d10282a7c"
    )

    const srg = find(required, "client-1.20.1-20230612.114412-srg.jar")
    expect(srg.expectedSha1).toBeUndefined()

    const lzma = find(required, "forge-1.20.1-forge-47.2.0-client.lzma")
    expect(lzma.expectedSha1).toBeUndefined()
  })

  it("neoforge 26.2.0.23-beta: required set without outputs or SHAs", () => {
    // Mirrors processor_outputs.rs's
    // neoforge_26_required_set_without_outputs_or_shas.
    const [procs, data] = parse(NEO_26_PROCESSORS, NEO_26_DATA)
    const required = requiredLibraryPaths(procs, data)

    expect(required).toHaveLength(2)
    const patched = find(
      required,
      "net/neoforged/minecraft-client-patched/26.2.0.23-beta/minecraft-client-patched-26.2.0.23-beta.jar"
    )
    expect(patched.expectedSha1).toBeUndefined()
    find(
      required,
      "net/minecraftforge/forge/neoforge-26.2.0.23-beta/forge-neoforge-26.2.0.23-beta-client.lzma"
    )
  })

  it("skips augmented and unknown data keys", () => {
    // Mirrors processor_outputs.rs's augmented_and_unknown_keys_are_skipped.
    const procs: Processor[] = JSON.parse(
      `[{"jar":"a:b:1","classpath":[],
         "args":["{MINECRAFT_JAR}","{ROOT}/libraries/","{SIDE}","{NOT_IN_DATA}","plain-arg"]}]`
    )
    expect(requiredLibraryPaths(procs, {})).toEqual([])
  })

  it("resolves direct maven-ref args without a data map", () => {
    // Mirrors processor_outputs.rs's direct_maven_ref_args_resolve_without_data.
    const procs: Processor[] = JSON.parse(
      `[{"jar":"a:b:1","classpath":[],
         "args":["[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412@zip]",
                 "[net.minecraftforge:forge:neoforge-26.1.0.0-alpha.1+snapshot-1:client@lzma]"]}]`
    )
    const required = requiredLibraryPaths(procs, undefined)
    expect(required).toHaveLength(2)
    find(
      required,
      "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412.zip"
    )
    find(
      required,
      "net/minecraftforge/forge/neoforge-26.1.0.0-alpha.1+snapshot-1/forge-neoforge-26.1.0.0-alpha.1+snapshot-1-client.lzma"
    )
    expect(required[0].expectedSha1).toBeUndefined()
  })

  it("yields an empty set for empty processors", () => {
    // Mirrors processor_outputs.rs's empty_processors_yield_empty_set.
    const [, data] = parse(FORGE_1_20_1_PROCESSORS, FORGE_1_20_1_DATA)
    expect(requiredLibraryPaths([], data)).toEqual([])
  })

  it("matches the real, live-fetched forge 1.20.1-47.4.22 shape (informational fixture)", () => {
    // Not asserted against a fixed count: real meta.gdl.gg payloads gain
    // processors over time (this one has 10, vs the trimmed fixture's 4)
    // and unrelated ones (MCP_DATA, DOWNLOAD_MOJMAPS, MERGE_MAPPING) resolve
    // to zero extra paths because their args reference keys absent from
    // `data` ({SIDE}) or the resolved refs collapse via `upsert`'s
    // deduplication. The real regression guard is that this does not throw
    // and does not return an empty set — confirmed by hand against a live
    // fetch.
    const realProcessors: Processor[] = JSON.parse(`[
      {"jar":"net.minecraftforge:installertools:1.4.1","classpath":[],
       "args":["--task","MCP_DATA","--input","[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412@zip]","--output","{MAPPINGS}"]},
      {"jar":"net.minecraftforge:installertools:1.4.1","classpath":[],
       "args":["--task","DOWNLOAD_MOJMAPS","--version","1.20.1","--side","{SIDE}","--output","{MOJMAPS}"]},
      {"jar":"net.minecraftforge:installertools:1.4.1","classpath":[],
       "args":["--task","MERGE_MAPPING","--left","{MAPPINGS}","--right","{MOJMAPS}","--output","{MERGED_MAPPINGS}"]},
      {"jar":"net.minecraftforge:jarsplitter:1.1.4","classpath":[],
       "args":["--input","{MINECRAFT_JAR}","--slim","{MC_SLIM}","--extra","{MC_EXTRA}"],
       "outputs":{"{MC_EXTRA}":"{MC_EXTRA_SHA}","{MC_SLIM}":"{MC_SLIM_SHA}"},
       "sides":["client"]},
      {"jar":"net.minecraftforge:ForgeAutoRenamingTool:0.1.22:all","classpath":[],
       "args":["--input","{MC_SLIM}","--output","{MC_SRG}","--names","{MERGED_MAPPINGS}"]},
      {"jar":"net.minecraftforge:binarypatcher:1.1.1","classpath":[],
       "args":["--clean","{MC_SRG}","--output","{PATCHED}","--apply","{BINPATCH}"]}
    ]`)
    const realData: Record<string, SidedDataEntry> = JSON.parse(`{
      "PATCHED":{"client":"[net.minecraftforge:forge:1.20.1-47.4.22:client]","server":"[net.minecraftforge:forge:1.20.1-47.4.22:server]"},
      "MAPPINGS":{"client":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings@txt]","server":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings@txt]"},
      "MOJMAPS":{"client":"[net.minecraft:client_mappings:1.20.1:mappings@txt]","server":"[net.minecraft:server_mappings:1.20.1:mappings@txt]"},
      "MERGED_MAPPINGS":{"client":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]","server":"[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings-merged@txt]"},
      "MC_SLIM":{"client":"[net.minecraft:client:1.20.1-20230612.114412:slim]","server":"[net.minecraft:server:1.20.1-20230612.114412:slim]"},
      "MC_SLIM_SHA":{"client":"'de86b035d2da0f78940796bb95c39a932ed84834'","server":"'ignored'"},
      "MC_EXTRA":{"client":"[net.minecraft:client:1.20.1-20230612.114412:extra]","server":"[net.minecraft:server:1.20.1-20230612.114412:extra]"},
      "MC_EXTRA_SHA":{"client":"'8c5a95cbce940cfdb304376ae9fea47968d02587'","server":"'ignored'"},
      "MC_SRG":{"client":"[net.minecraft:client:1.20.1-20230612.114412:srg]","server":"[net.minecraft:server:1.20.1-20230612.114412:srg]"},
      "BINPATCH":{"client":"[net.minecraftforge:forge:1.20.1-forge-47.4.22:client@lzma]","server":"[net.minecraftforge:forge:1.20.1-forge-47.4.22:server@lzma]"}
    }`)

    const required = requiredLibraryPaths(realProcessors, realData)
    expect(required.length).toBeGreaterThan(0)
    find(required, "forge-1.20.1-47.4.22-client.jar")
    find(required, "mcp_config-1.20.1-20230612.114412-mappings-merged.txt")
    // {SIDE} has no entry in `data` — DOWNLOAD_MOJMAPS's --side arg must not
    // fabricate a path or throw.
    expect(required.some((f) => f.relativePath.includes("undefined"))).toBe(
      false
    )
  })
})
