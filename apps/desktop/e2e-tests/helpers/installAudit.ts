/**
 * Parses `.install_audit/audit.txt`, the plain-text record
 * `process_modpack_staging` writes of every decision a modpack install or
 * version change made (`crates/carbon_app/src/managers/instance/run/modpack.rs:833-892`).
 *
 * This is the single best oracle in the whole feature: it names, per file,
 * whether the pass replaced it, deleted it, created it, or refused to touch
 * it and why. Asserting against it means a test can prove *why* a file
 * survived, not just that it did.
 *
 * The directory is deleted and recreated on every pass, so its content
 * always describes the most recent one only — never an accumulation.
 *
 * Two deliberate non-normalisations. First, `null` (no audit directory)
 * means "the pass never ran" and is kept distinct from an audit with four
 * empty sections, which means "the pass ran and decided nothing"; collapsing
 * them would make a skipped staging phase look like a no-op one. Second, the
 * three packinfo-derived sections carry keys with a leading `/` while
 * `Files created:` carries staging-relative paths with no leading slash and
 * an `instance/` prefix. That difference is real (`modpack.rs:812-819`), so
 * it is preserved here and normalised at the comparison site.
 */

import fs from "node:fs/promises"
import path from "node:path"

export type SkipReason =
  | "deleted-by-user"
  | "modified-by-user"
  | "in-save-folder"

export interface AuditSkip {
  file: string
  reason: SkipReason
  /** Present only on `modified-by-user`. */
  originalMd5?: string
  /** Present only on `modified-by-user`. */
  currentMd5?: string
}

export interface InstallAudit {
  skipped: AuditSkip[]
  deleted: string[]
  replaced: string[]
  created: string[]
}

const SECTIONS = {
  "Files that could not be replaced:": "skipped",
  "Files deleted:": "deleted",
  "Files replaced:": "replaced",
  "Files created:": "created"
} as const

const REASONS: Record<string, SkipReason> = {
  "deleted by user": "deleted-by-user",
  "modified by user": "modified-by-user",
  "files in /saves will never be modified": "in-save-folder"
}

export function parseInstallAudit(text: string): InstallAudit {
  const audit: InstallAudit = {
    skipped: [],
    deleted: [],
    replaced: [],
    created: []
  }

  let section: keyof typeof audit | undefined

  for (const line of text.split("\n")) {
    const heading = SECTIONS[line.trim() as keyof typeof SECTIONS]
    if (heading) {
      section = heading
      continue
    }

    if (line.startsWith("     original md5: ")) {
      const last = audit.skipped[audit.skipped.length - 1]
      if (!last) throw new Error(`orphaned md5 continuation line: ${line}`)
      last.originalMd5 = line.slice("     original md5: ".length).trim()
      continue
    }
    if (line.startsWith("     current md5:  ")) {
      const last = audit.skipped[audit.skipped.length - 1]
      if (!last) throw new Error(`orphaned md5 continuation line: ${line}`)
      last.currentMd5 = line.slice("     current md5:  ".length).trim()
      continue
    }

    if (!line.startsWith(" - ") || !section) continue
    const body = line.slice(" - ".length)

    if (section !== "skipped") {
      audit[section].push(body)
      continue
    }

    // Split on the LAST colon-space, not the first: a filename may contain a
    // colon and the reason never does.
    const cut = body.lastIndexOf(": ")
    if (cut === -1) throw new Error(`unparseable audit skip line: ${line}`)
    const file = body.slice(0, cut)
    const rawReason = body.slice(cut + 2)
    const reason = REASONS[rawReason]
    if (!reason) {
      throw new Error(
        `unrecognised audit skip reason "${rawReason}" on line: ${line}`
      )
    }
    audit.skipped.push({ file, reason })
  }

  return audit
}

export async function readInstallAudit(
  instanceRoot: string
): Promise<InstallAudit | null> {
  const file = path.join(instanceRoot, ".install_audit", "audit.txt")
  try {
    return parseInstallAudit(await fs.readFile(file, "utf8"))
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") return null
    throw err
  }
}
