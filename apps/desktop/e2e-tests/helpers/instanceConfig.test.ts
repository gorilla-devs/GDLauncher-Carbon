import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { readInstanceConfig } from "./instanceConfig.js"

let instanceRoot: string
let tmpRoot: string

beforeEach(async () => {
  tmpRoot = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-e2e-instanceconfig-"))
  instanceRoot = path.join(tmpRoot, "instance-shortpath")
  await fs.mkdir(instanceRoot, { recursive: true })
})

afterEach(async () => {
  await fs.rm(tmpRoot, { recursive: true, force: true })
})

async function writeConfig(body: unknown): Promise<void> {
  await fs.writeFile(
    path.join(instanceRoot, "instance.json"),
    JSON.stringify(body),
    "utf8"
  )
}

describe("readInstanceConfig", () => {
  it("reads seconds_played, defaulting to 0 when the key is absent", async () => {
    await writeConfig({
      _version: "1",
      name: "gdl-e2e-played",
      seconds_played: 4242,
      game_configuration: { version: { release: "1.20.1", modloaders: [] } }
    })
    expect((await readInstanceConfig(instanceRoot)).secondsPlayed).toBe(4242)

    // `#[serde(default)]` on the Rust side: an instance that has never been
    // played can legitimately omit the key, and that is 0 rather than a
    // malformed file.
    await writeConfig({
      _version: "1",
      name: "gdl-e2e-played",
      game_configuration: { version: { release: "1.20.1", modloaders: [] } }
    })
    expect((await readInstanceConfig(instanceRoot)).secondsPlayed).toBe(0)
  })

  it("reads name and release version off a standard v1 config", async () => {
    await writeConfig({
      _version: "1",
      name: "gdl-e2e-persistence",
      game_configuration: {
        version: {
          release: "1.20.1",
          modloaders: [{ type_: "Fabric", version: "0.15.11" }]
        },
        global_java_args: true
      }
    })

    const result = await readInstanceConfig(instanceRoot)
    expect(result.name).toBe("gdl-e2e-persistence")
    expect(result.mcVersion).toBe("1.20.1")
    expect(result.modloaders).toEqual([{ type_: "Fabric", version: "0.15.11" }])
  })

  it("reads mcVersion as null for a Custom (bare-string) version", async () => {
    await writeConfig({
      _version: "1",
      name: "custom-instance",
      game_configuration: {
        version: "some-custom-version-string",
        global_java_args: true
      }
    })

    const result = await readInstanceConfig(instanceRoot)
    expect(result.name).toBe("custom-instance")
    expect(result.mcVersion).toBeNull()
    expect(result.modloaders).toEqual([])
  })

  it("reads mcVersion as null when no version is set at all", async () => {
    await writeConfig({
      _version: "1",
      name: "no-version-instance",
      game_configuration: {
        global_java_args: true
      }
    })

    const result = await readInstanceConfig(instanceRoot)
    expect(result.mcVersion).toBeNull()
  })

  it("throws (never a vacuous result) when the config file is missing", async () => {
    await expect(readInstanceConfig(instanceRoot)).rejects.toThrow(
      /could not read/
    )
  })

  it("throws when the config file is not valid JSON", async () => {
    await fs.writeFile(
      path.join(instanceRoot, "instance.json"),
      "not json {{{",
      "utf8"
    )

    await expect(readInstanceConfig(instanceRoot)).rejects.toThrow(
      /not valid JSON/
    )
  })

  it("throws when the config file has no string name field", async () => {
    await writeConfig({ _version: "1", game_configuration: {} })

    await expect(readInstanceConfig(instanceRoot)).rejects.toThrow(
      /no string "name" field/
    )
  })

  it("reads modpack as null for a plain (non-modpack) instance", async () => {
    await writeConfig({
      _version: "1",
      name: "plain-instance",
      game_configuration: {}
    })

    expect((await readInstanceConfig(instanceRoot)).modpack).toBeNull()
  })

  it("reads a Modrinth modpack — flat, no modpack/value nesting", async () => {
    await writeConfig({
      _version: "1",
      name: "mr-pack-instance",
      game_configuration: {},
      // The real wire shape (v1::ModpackInfo #[serde(flatten)]-ing
      // v1::Modpack, tag = "platform", no `content`): platform/project_id/
      // version_id/locked all sit on one flat object, never nested under a
      // second "modpack" key or a "value" wrapper — see this module's own
      // doc comment for why that's easy to get wrong by analogy with
      // GameResolution's `tag = "type", content = "value"` sibling enum.
      modpack: {
        platform: "Modrinth",
        project_id: "MNW3LUwK",
        version_id: "eGIPjEwN",
        locked: true
      }
    })

    expect((await readInstanceConfig(instanceRoot)).modpack).toEqual({
      platform: "modrinth",
      modrinthProjectId: "MNW3LUwK",
      modrinthVersionId: "eGIPjEwN",
      curseforgeProjectId: null,
      curseforgeFileId: null,
      locked: true
    })
  })

  it("reads a CurseForge modpack — numeric project/file ids", async () => {
    await writeConfig({
      _version: "1",
      name: "cf-pack-instance",
      game_configuration: {},
      modpack: {
        platform: "Curseforge",
        project_id: 520990,
        file_id: 4713831,
        locked: false
      }
    })

    expect((await readInstanceConfig(instanceRoot)).modpack).toEqual({
      platform: "curseforge",
      modrinthProjectId: null,
      modrinthVersionId: null,
      curseforgeProjectId: 520990,
      curseforgeFileId: 4713831,
      locked: false
    })
  })

  it("defaults locked to false when the key is absent", async () => {
    await writeConfig({
      _version: "1",
      name: "unlocked-by-omission",
      game_configuration: {},
      modpack: { platform: "Modrinth", project_id: "abc", version_id: "def" }
    })

    expect((await readInstanceConfig(instanceRoot)).modpack?.locked).toBe(false)
  })

  it("throws when modpack.platform is neither Curseforge nor Modrinth", async () => {
    await writeConfig({
      _version: "1",
      name: "bad-platform-instance",
      game_configuration: {},
      modpack: { platform: "Technic", project_id: "abc" }
    })

    await expect(readInstanceConfig(instanceRoot)).rejects.toThrow(
      /unrecognised "platform"/
    )
  })
})
