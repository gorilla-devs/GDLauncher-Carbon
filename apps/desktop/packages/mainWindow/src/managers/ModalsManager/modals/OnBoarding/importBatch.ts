import { selectedImport, type ScanRow } from "./importSelection"

interface Batch {
  session: string
  selectionIds: string[]
  rows: ScanRow[]
  submit: (request: ReturnType<typeof selectedImport>) => Promise<unknown>
  currentSession: () => string | undefined
  setBusy: (value: boolean) => void
  removeSelections: (ids: string[]) => void
  onUncertain: (errors: unknown[]) => void
  onSuccess: () => void
}

export function createImportBatch() {
  let active = false
  // A mutation rejection may follow publication. Its row stays consumed until
  // an explicit new scan supplies a new session, just like a successful row.
  const outcomes = new Map<string, "pending" | "success" | "uncertain">()
  return async (batch: Batch) => {
    if (active || batch.currentSession() !== batch.session) return
    const ids = [...new Set(batch.selectionIds)]
    if (ids.length === 0) return
    let requests: ReturnType<typeof selectedImport>[]
    try {
      requests = ids.map((id) => selectedImport(batch.rows, id))
      if (
        requests.some((request) => request.session !== batch.session) ||
        ids.some((id) => outcomes.has(id))
      ) {
        throw new Error("This selection requires a fresh scan")
      }
    } catch (error) {
      batch.onUncertain([error])
      return
    }
    active = true
    batch.setBusy(true)
    ids.forEach((id) => outcomes.set(id, "pending"))
    try {
      const results = await Promise.allSettled(
        requests.map(async (request) => batch.submit(request))
      )
      results.forEach((result, index) =>
        outcomes.set(
          ids[index],
          result.status === "fulfilled" ? "success" : "uncertain"
        )
      )
      if (batch.currentSession() !== batch.session) return
      batch.removeSelections(ids)
      const errors = results.flatMap((result) =>
        result.status === "rejected" ? [result.reason] : []
      )
      if (errors.length) batch.onUncertain(errors)
      else batch.onSuccess()
    } finally {
      active = false
      batch.setBusy(false)
    }
  }
}
