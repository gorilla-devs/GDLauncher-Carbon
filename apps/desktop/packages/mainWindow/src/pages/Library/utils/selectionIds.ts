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

export function parseServerIds(stringIds: Iterable<string>): number[] {
  return Array.from(stringIds)
    .filter((id) => /^server-\d+$/.test(id))
    .map((id) => parseInt(id.replace("server-", ""), 10))
}

export function parseServerFolderIds(stringIds: Iterable<string>): number[] {
  return Array.from(stringIds)
    .filter((id) => /^server-folder-\d+$/.test(id))
    .map((id) => Number(id.slice("server-folder-".length)))
}
