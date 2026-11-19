/**
 * The name a rename box asks for: what was typed without the spaces around
 * it, or null when that is blank or the name the instance already has, so
 * nothing is sent. The launcher refuses a blank name too; a name that only
 * differs by the spaces around it is no rename.
 */
export function renameTo(typed: string, current: string | undefined) {
  const name = typed.trim()
  return name === "" || name === current?.trim() ? null : name
}
