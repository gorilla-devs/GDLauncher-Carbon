import type {
  CleanupSelection,
  FEUntrackedFile
} from "@gd/core_module/bindings"

export function reconcileSelections(
  selected: Map<string, CleanupSelection>,
  rows: FEUntrackedFile[]
): Map<string, CleanupSelection> {
  const next = new Map<string, CleanupSelection>()
  for (const row of rows) {
    const previous = selected.get(row.path)
    const current = row.selection
    if (
      previous &&
      current &&
      row.deletable &&
      previous.sha512 === current.sha512 &&
      previous.proven === current.proven &&
      previous.context.platform === current.context.platform &&
      previous.context.project_id === current.context.project_id &&
      previous.context.version_id === current.context.version_id
    ) {
      next.set(row.path, previous)
    }
  }
  return next
}

export function freezeSelection(selection: CleanupSelection): CleanupSelection {
  return { ...selection, context: { ...selection.context } }
}
