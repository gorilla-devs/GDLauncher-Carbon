import fs from "node:fs/promises"
import path from "node:path"
import { randomUUID } from "node:crypto"
import { LEFT_BEHIND_SHOWN, type RuntimeMoveNotice } from "./runtimeRelocation"

// Next to runtime_path_override in userData, which a move never touches.
export const RUNTIME_MOVE_NOTICE_NAME = "runtime_move_notice.json"

/**
 * Keeps what a finished move left in the old folder, shown in the user's language on
 * every start from the new one until the user acknowledges it; null forgets an earlier
 * move's notice. Main restarts right after, so the notice cannot depend on a window
 * being there to read it. Never throws: the move is complete either way and only the
 * notice is lost.
 */
export async function recordRuntimeMoveNotice(
  userData: string,
  notice: RuntimeMoveNotice | null
): Promise<void> {
  const file = path.join(userData, RUNTIME_MOVE_NOTICE_NAME)
  if (!notice) {
    await fs.unlink(file).catch(() => {})
    return
  }
  const temporary = path.join(
    userData,
    `.${RUNTIME_MOVE_NOTICE_NAME}.${randomUUID()}.tmp`
  )
  try {
    await fs.writeFile(temporary, JSON.stringify({ version: 1, ...notice }), {
      flag: "wx",
      mode: 0o600
    })
    await fs.rename(temporary, file)
  } catch (error) {
    console.warn(
      `Could not keep the notice about files left in the old data folder: ${String(error)}`
    )
    await fs.unlink(temporary).catch(() => {})
  }
}

function fileList(value: unknown): string[] | null {
  return Array.isArray(value) && value.every((v) => typeof v === "string")
    ? value.slice(0, LEFT_BEHIND_SHOWN)
    : null
}

function count(value: unknown, shown: string[]): number | null {
  return Number.isSafeInteger(value) && (value as number) >= shown.length
    ? (value as number)
    : null
}

function parseNotice(value: unknown): RuntimeMoveNotice | null {
  if (!value || typeof value !== "object") return null
  const v = value as Record<string, unknown>
  const changed = fileList(v.changed),
    notDeleted = fileList(v.notDeleted)
  if (
    v.version !== 1 ||
    typeof v.oldFolder !== "string" ||
    typeof v.newFolder !== "string" ||
    typeof v.keptOldFolder !== "boolean" ||
    !changed ||
    !notDeleted
  )
    return null
  const changedCount = count(v.changedCount, changed),
    notDeletedCount = count(v.notDeletedCount, notDeleted)
  if (changedCount === null || notDeletedCount === null) return null
  return {
    oldFolder: v.oldFolder,
    newFolder: v.newFolder,
    changed,
    changedCount,
    notDeleted,
    notDeletedCount,
    keptOldFolder: v.keptOldFolder
  }
}

/**
 * The notice about the move into the data folder in use. Despite the name, a readable
 * notice is not removed here: it stays until the user acknowledges it
 * (dismissRuntimeMoveNotice), so a reload or a modal that failed to open shows it
 * again: it is the only list of what may be newer in the old folder. A notice for
 * another folder (an explicit data path override is in effect) waits.
 */
export async function takeRuntimeMoveNotice(
  userData: string,
  runtimePath: string
): Promise<RuntimeMoveNotice | null> {
  const file = path.join(userData, RUNTIME_MOVE_NOTICE_NAME)
  let notice: RuntimeMoveNotice | null
  try {
    notice = parseNotice(JSON.parse(await fs.readFile(file, "utf8")))
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null
    notice = null
  }
  // Unreadable, so never going to be shown.
  if (!notice) await fs.unlink(file).catch(() => {})
  if (notice && path.resolve(notice.newFolder) !== path.resolve(runtimePath))
    return null
  return notice
}

/**
 * The user acknowledged the notice, so it is not shown again. Never throws: a notice
 * that cannot be removed is shown again on the next start.
 */
export async function dismissRuntimeMoveNotice(
  userData: string
): Promise<void> {
  try {
    await fs.unlink(path.join(userData, RUNTIME_MOVE_NOTICE_NAME))
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT")
      console.warn(
        `Could not remove the notice about files left in the old data folder: ${String(error)}`
      )
  }
}
