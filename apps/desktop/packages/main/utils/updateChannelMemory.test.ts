import { afterEach, beforeEach, describe, expect, it } from "vitest"
import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import {
  UPDATE_CHANNEL_NAME,
  buildChannel,
  recallUpdateChannel,
  rememberUpdateChannel
} from "./updateChannelMemory"

let userData: string
beforeEach(async () => {
  userData = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-update-channel-"))
})
afterEach(() => fs.rm(userData, { recursive: true, force: true }))

describe("the update channel kept for when the core cannot start", () => {
  it("reads a build's channel from its version", () => {
    expect(buildChannel("2.0.41")).toBe("stable")
    expect(buildChannel("2.0.41-beta.3")).toBe("beta")
    expect(buildChannel("2.0.41-alpha.1")).toBe("alpha")
  })
  it("recalls the last channel remembered", async () => {
    expect(await recallUpdateChannel(userData)).toBeNull()
    await rememberUpdateChannel(userData, "beta")
    await rememberUpdateChannel(userData, "alpha")
    expect(await recallUpdateChannel(userData)).toBe("alpha")
    expect(await fs.readdir(userData)).toEqual([UPDATE_CHANNEL_NAME])
  })
  it.each(["", "not json", "null", '{"version":1,"channel":"nightly"}'])(
    "recalls nothing from an unreadable file: %s",
    async (content) => {
      await fs.writeFile(path.join(userData, UPDATE_CHANNEL_NAME), content)
      expect(await recallUpdateChannel(userData)).toBeNull()
    }
  )
  it("does not throw when the folder is gone", async () => {
    await fs.rm(userData, { recursive: true })
    await rememberUpdateChannel(userData, "beta")
    expect(await recallUpdateChannel(userData)).toBeNull()
  })
})
