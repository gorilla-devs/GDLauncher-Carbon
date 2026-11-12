import fs from "node:fs/promises"
import path from "node:path"
import { randomUUID } from "node:crypto"
import type { FEReleaseChannel } from "@gd/core_module/bindings"

// In userData, which a data folder move never touches.
export const UPDATE_CHANNEL_NAME = "update_channel.json"

const CHANNELS: readonly FEReleaseChannel[] = ["stable", "beta", "alpha"]

export function isReleaseChannel(value: unknown): value is FEReleaseChannel {
  return CHANNELS.includes(value as FEReleaseChannel)
}

/** The channel a build was published on, read from its version. */
export function buildChannel(version: string): FEReleaseChannel {
  return version.includes("alpha")
    ? "alpha"
    : version.includes("beta")
      ? "beta"
      : "stable"
}

/**
 * Keeps the release channel the user last checked for updates on. The setting itself
 * lives in the core's database, which the screen shown when the core cannot start has
 * no way to read. Never throws: only the memory is lost.
 */
export async function rememberUpdateChannel(
  userData: string,
  channel: FEReleaseChannel
): Promise<void> {
  const temporary = path.join(
    userData,
    `.${UPDATE_CHANNEL_NAME}.${randomUUID()}.tmp`
  )
  try {
    await fs.writeFile(temporary, JSON.stringify({ version: 1, channel }), {
      flag: "wx",
      mode: 0o600
    })
    await fs.rename(temporary, path.join(userData, UPDATE_CHANNEL_NAME))
  } catch (error) {
    console.warn(`Could not keep the update channel: ${String(error)}`)
    await fs.unlink(temporary).catch(() => {})
  }
}

/** The remembered channel, or null when none was kept or it cannot be read. */
export async function recallUpdateChannel(
  userData: string
): Promise<FEReleaseChannel | null> {
  try {
    const value: unknown = JSON.parse(
      await fs.readFile(path.join(userData, UPDATE_CHANNEL_NAME), "utf8")
    )
    const channel = (value as { version?: unknown; channel?: unknown } | null)
      ?.channel
    return (value as { version?: unknown }).version === 1 &&
      isReleaseChannel(channel)
      ? channel
      : null
  } catch {
    return null
  }
}
