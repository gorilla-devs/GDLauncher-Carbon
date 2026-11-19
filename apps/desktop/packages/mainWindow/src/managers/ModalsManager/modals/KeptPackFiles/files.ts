/** A kept file's name and the folder it is in, both as the modpack spells
 * them ("config/jei/jei-client.ini" is "jei-client.ini" in "config/jei"). A
 * file directly in the game folder has no folder to name. */
export function splitPath(path: string): { name: string; folder: string } {
  const cut = path.lastIndexOf("/")
  return cut < 0
    ? { name: path, folder: "" }
    : { name: path.slice(cut + 1), folder: path.slice(0, cut) }
}

/** The files a replacement left the player's, given what the core answered
 * for each one asked: the ones it could not replace, and the ones the player
 * did not ask for. */
export function remaining(
  files: readonly string[],
  results: readonly { path: string; error: string | null }[]
): string[] {
  const replaced = new Set(
    results.filter((result) => result.error === null).map(({ path }) => path)
  )
  return files.filter((path) => !replaced.has(path))
}
