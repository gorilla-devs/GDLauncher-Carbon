import fs from "node:fs/promises"
import path from "node:path"
import { createHash } from "node:crypto"
import type { TreeEntry } from "./instanceTree.js"
import { packinfoDataPath, readPackinfo } from "./packinfo.js"

export interface RunningRepairProtection {
  artifacts: ReadonlyMap<string, TreeEntry>
  record: Buffer
}

async function requireAbsent(file: string, message: string): Promise<void> {
  try {
    await fs.lstat(file)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return
    throw error
  }
  throw new Error(message)
}

async function readArtifact(root: string, key: string): Promise<Buffer> {
  const file = packinfoDataPath(root, key)
  try {
    if (!(await fs.lstat(file)).isFile())
      throw new Error("Expected a regular active JAR")
    await requireAbsent(
      `${file}.disabled`,
      `Protected artifact has a disabled twin: ${key}`
    )
    return await fs.readFile(file)
  } catch (cause) {
    throw new Error(`Protected pack artifact unavailable: ${key}`, { cause })
  }
}

/** Fresh active/pristine fixture only; normal disabled-pack semantics are separate.
 * Retain every pack-owned mods JAR before confirmation, never refilter afterward.
 * Game-written logs/configs/worlds are outside this bounded preservation proof.
 */
export async function captureRunningRepairProtection(
  root: string
): Promise<RunningRepairProtection> {
  await requireAbsent(
    path.join(root, ".setup"),
    "Running fixture already has .setup"
  )
  const record = await fs.readFile(path.join(root, "packinfo.json"))
  const info = await readPackinfo(root)
  const artifacts = new Map<string, TreeEntry>()
  for (const [key, entry] of info) {
    if (!key.startsWith("/mods/") || !key.endsWith(".jar")) continue
    if (
      key
        .slice(1)
        .split(/[\\/]/)
        .some((part) => !part || part === "." || part === "..")
    )
      throw new Error(`Invalid protected artifact target: ${key}`)
    const bytes = await readArtifact(root, key)
    if (
      createHash("md5").update(bytes).digest("hex") !==
        entry.md5.toLowerCase() ||
      createHash("sha512").update(bytes).digest("hex") !==
        entry.sha512.toLowerCase()
    )
      throw new Error(
        `Protected artifact is not pristine in this fresh fixture: ${key}`
      )
    artifacts.set(key, {
      size: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex")
    })
  }
  if (artifacts.size === 0)
    throw new Error(
      "Running fixture requires a nonempty protected pack-owned JAR set"
    )
  if (!(await fs.readFile(path.join(root, "packinfo.json"))).equals(record))
    throw new Error("Packinfo record changed while retaining protected targets")
  return { artifacts, record }
}

export async function verifyRunningRepairProtection(
  root: string,
  before: RunningRepairProtection
): Promise<void> {
  await requireAbsent(
    path.join(root, ".setup"),
    "Repair refusal created .setup"
  )
  if (
    !(await fs.readFile(path.join(root, "packinfo.json"))).equals(before.record)
  )
    throw new Error("Repair refusal changed the exact packinfo record")
  for (const [key, expected] of before.artifacts) {
    const bytes = await readArtifact(root, key)
    if (
      bytes.length !== expected.size ||
      createHash("sha256").update(bytes).digest("hex") !== expected.sha256
    )
      throw new Error(
        `Repair refusal changed protected pack artifact bytes: ${key}`
      )
  }
}
