/**
 * Parses `.install_audit/audit.txt`, the plain-text record
 * `process_modpack_staging` writes of every decision a modpack install,
 * version change or repair made — rendered by the pure `render_audit`
 * (`crates/carbon_app/src/managers/instance/run/modpack.rs`), which builds it
 * from the apply planner's `PlanEntry` list (`managers/instance/modpack/apply_plan.rs`).
 *
 * This is the single best oracle in the whole feature: it names, per file,
 * whether the pass replaced it, deleted it, created it, left it alone, or
 * refused to touch it and why. Asserting against it means a test can prove
 * *why* a file survived, not just that it did.
 *
 * The directory is deleted and recreated on every pass, so its content
 * always describes the most recent one only — never an accumulation.
 *
 * One deliberate non-normalisation: `null` (no audit directory) means "the
 * pass never ran" and is kept distinct from an audit with every section
 * empty, which means "the pass ran and decided nothing"; collapsing them
 * would make a skipped staging phase look like a no-op one.
 *
 * Every section carries packinfo-style keys with a leading `/`, including
 * `Files created:` — `render_audit` writes the plan's own `path` field
 * everywhere, so there is no more staging-relative/packinfo-style split to
 * normalise away. A caller that still does
 * `.replace(/^instance\//, "")` on a `created` entry is unaffected: the
 * prefix it strips no longer appears, so the replace is a no-op.
 */

import fs from "node:fs/promises"
import path from "node:path"

export type SkipReason =
  | "deleted-by-user"
  | "modified-by-user"
  | "in-save-folder"
  | "disabled-by-user"
  | "already-present"

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
  /** Left untouched because it already matched the target — `Keep`/`Unchanged`
   *  entries that don't fall into one of the `skipped` reasons above. */
  unchanged: string[]
  /** A disabled twin (`*.jar.disabled`) restored to its enabled path by a
   *  repair. Repair-only — a plain version change never produces this. */
  reEnabled: string[]
  /** Explicitly removed at the user's request (a future prune/repair
   *  feature) rather than by the pack's own reconciliation. */
  userRemoved: string[]
}

const SECTIONS = {
  "Files that could not be replaced:": "skipped",
  "Files deleted:": "deleted",
  "Files replaced:": "replaced",
  "Files created:": "created",
  "Files unchanged:": "unchanged",
  "Files re-enabled:": "reEnabled",
  "Files removed at user request:": "userRemoved"
} as const

const REASONS: Record<string, SkipReason> = {
  "deleted by user": "deleted-by-user",
  "modified by user": "modified-by-user",
  "files in /saves will never be modified": "in-save-folder",
  "disabled by user": "disabled-by-user",
  "already present": "already-present"
}

export function parseInstallAudit(text: string): InstallAudit {
  const audit: InstallAudit = {
    skipped: [],
    deleted: [],
    replaced: [],
    created: [],
    unchanged: [],
    reEnabled: [],
    userRemoved: []
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
