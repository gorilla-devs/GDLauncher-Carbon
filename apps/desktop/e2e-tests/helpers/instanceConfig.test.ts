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
})
