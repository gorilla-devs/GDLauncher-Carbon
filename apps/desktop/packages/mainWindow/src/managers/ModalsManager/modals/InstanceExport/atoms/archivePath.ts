const suffixes = {
  Curseforge: ".zip",
  Gdlauncher: ".gdlpack",
  Modrinth: ".mrpack"
} as const

export function archiveSuffix(target: string): string | undefined {
  return Object.hasOwn(suffixes, target)
    ? suffixes[target as keyof typeof suffixes]
    : undefined
}

export function normalizeArchivePath(path: string, target: string): string {
  const suffix = archiveSuffix(target)
  if (!path || !suffix) return path
  const previous = Object.values(suffixes).find((candidate) =>
    path.endsWith(candidate)
  )
  return previous ? path.slice(0, -previous.length) + suffix : path + suffix
}

export function validArchivePath(
  path: unknown,
  target: string
): path is string {
  const suffix = archiveSuffix(target)
  return (
    typeof path === "string" &&
    !!suffix &&
    path.length > suffix.length &&
    path.endsWith(suffix)
  )
}
