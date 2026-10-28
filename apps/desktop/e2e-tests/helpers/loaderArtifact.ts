import fs from "node:fs/promises"
import path from "node:path"
import { createHash } from "node:crypto"
import type { CachedPartialVersionInfo } from "./versionCache.js"
import type { VerifyResult } from "./installVerify.js"
import { mavenCoordinateToPath } from "./processorOutputs.js"
/** Select the legacy runtime, never its installer or unrelated dependency.
 * Bare and universal coordinates are supported; a second candidate is ambiguous.
 */
export async function verifyLegacyForgeRuntime(
  runtimePath: string,
  cacheId: string,
  loaderVersion: string,
  profile: CachedPartialVersionInfo
): Promise<VerifyResult> {
  const fail = (problem: string): VerifyResult => ({
    ok: false,
    problems: [problem]
  })
  if (cacheId !== `forge-${loaderVersion}`)
    return fail("Selected Forge cache identity does not match")
  const prefix = `net.minecraftforge:forge:${loaderVersion}`
  const matches = (profile.libraries ?? []).filter(
    (lib) => lib.name === prefix || lib.name === `${prefix}:universal`
  )
  if (matches.length !== 1)
    return fail(
      `Expected one selected Forge runtime in ${cacheId}; found ${matches.length}`
    )
  const library = matches[0]
  const artifact = library.downloads?.artifact
  const relative = artifact?.path ?? mavenCoordinateToPath(library.name)
  if (
    !relative ||
    path.isAbsolute(relative) ||
    relative.split(/[\\/]/).some((part) => part === "..") ||
    !relative.endsWith(".jar")
  )
    return fail("Invalid selected Forge runtime path")
  const file = path.join(runtimePath, "libraries", relative)
  try {
    if (!(await fs.lstat(file)).isFile())
      return fail(`Selected Forge runtime is not a regular file: ${file}`)
    const bytes = await fs.readFile(file)
    if (artifact?.size !== undefined && bytes.length !== artifact.size)
      return fail(`Selected Forge runtime has wrong size: ${file}`)
    if (artifact?.sha1) {
      const actual = createHash("sha1").update(bytes).digest("hex")
      if (actual !== artifact.sha1.toLowerCase())
        return fail(`Selected Forge runtime has wrong SHA1: ${file}`)
    }
    return { ok: true, problems: [] }
  } catch (error) {
    return fail(
      `Selected Forge runtime unavailable at ${file}: ${String(error)}`
    )
  }
}
