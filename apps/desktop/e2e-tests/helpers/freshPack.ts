import fs from "node:fs/promises"
import path from "node:path"
import { createHash } from "node:crypto"
import type { PackIndex } from "./modpacks.js"
import type { Tree } from "./instanceTree.js"
/** Fresh fixture's declared-content and independent effective-override assertions. */
export async function verifyFreshPack(
  instanceRoot: string,
  index: PackIndex,
  tree: Tree
): Promise<void> {
  const overrides = new Set(index.overrideFiles.map((file) => file.path))
  for (const file of index.files) {
    if (overrides.has(file.path))
      throw new Error(
        `Unsupported declared/override collision in fresh fixture: ${file.path}`
      )
  }
  for (const file of index.files) {
    const entry = tree.get(file.path)
    if (entry?.size !== file.size)
      throw new Error(`Pack file missing or wrong size: ${file.path}`)
    const bytes = await fs.readFile(
      path.join(instanceRoot, "instance", file.path)
    )
    if (createHash("sha512").update(bytes).digest("hex") !== file.sha512)
      throw new Error(`Wrong declared SHA512: ${file.path}`)
  }
  for (const override of index.overrideFiles) {
    const physical = tree.get(override.path)
    if (physical?.sha256 !== override.sha256)
      throw new Error(`Override missing or wrong SHA256: ${override.path}`)
  }
}
