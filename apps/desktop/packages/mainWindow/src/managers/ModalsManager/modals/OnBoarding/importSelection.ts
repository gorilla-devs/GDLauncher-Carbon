import type {
  ImportEntry,
  ImportableInstance,
  InvalidImportEntry
} from "@gd/core_module/bindings"
export type ScanRow = (ImportableInstance | InvalidImportEntry) & {
  backendIndex: number
  session: string
  selectionId: string
}
export function scanRows(entries: ImportEntry[], session: string): ScanRow[] {
  return entries
    .map((entry, backendIndex) => ({
      ...("Valid" in entry ? entry.Valid : entry.Invalid),
      backendIndex,
      session,
      selectionId: `${session}:${backendIndex}`
    }))
    .sort((a, b) => {
      const aValid = "filename" in a
      const bValid = "filename" in b
      if (aValid !== bValid) return aValid ? -1 : 1
      return (aValid ? a.instance_name : a.name).localeCompare(
        bValid ? b.instance_name : b.name,
        undefined,
        { sensitivity: "base" }
      )
    })
}
export function selectedImport(rows: ScanRow[], selectionId: string) {
  const entry = rows.find(
    (row) => "filename" in row && row.selectionId === selectionId
  )
  if (!entry || !("filename" in entry))
    throw new Error("Selected import is unavailable; rescan")
  return {
    index: entry.backendIndex,
    name: entry.instance_name,
    session: entry.session
  }
}
