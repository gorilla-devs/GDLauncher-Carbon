export function parseInstanceIds(stringIds: Iterable<string>): number[] {
  return Array.from(stringIds)
    .filter((id) => id.startsWith("instance-"))
    .map((id) => parseInt(id.replace("instance-", ""), 10))
}

export function parseFolderIds(stringIds: Iterable<string>): number[] {
  return Array.from(stringIds)
    .filter((id) => id.startsWith("folder-"))
    .map((id) => parseInt(id.replace("folder-", ""), 10))
}
