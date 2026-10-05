import { Accessor, createEffect, createSignal, onCleanup } from "solid-js"
import type { GameResolution } from "@gd/core_module/bindings"

type Resolution = GameResolution | null
type Owner = object

// Resolution alone owns this publication queue. Captured user saves survive
// navigation, but only the current page may reconcile its editable draft.
let publication: Promise<void> = Promise.resolve()
let revision = 0
let dirty = false
let pendingDraft: Resolution | undefined
let failedRevision: number | undefined
let currentOwner: Owner | undefined

const copy = (value: Resolution): Resolution =>
  value === null
    ? null
    : {
        type: value.type,
        value: [value.value[0], value.value[1]]
      }

export function createResolutionDraft(options: {
  read: Accessor<Resolution | undefined>
  save: (_value: Resolution) => Promise<unknown>
  refresh: (_owns: () => boolean) => Promise<Resolution | undefined>
}) {
  const owner: Owner = {}
  currentOwner = owner
  let live = true
  const initial = dirty ? pendingDraft : options.read()
  const [draft, setDraft] = createSignal<Resolution | undefined>(
    initial === undefined ? undefined : copy(initial)
  )
  const [busy, setBusy] = createSignal(dirty && failedRevision !== revision)
  const [error, setError] = createSignal(dirty && failedRevision === revision)
  const owns = () => live && currentOwner === owner

  onCleanup(() => {
    live = false
    if (currentOwner === owner) currentOwner = undefined
  })

  createEffect(() => {
    const value = options.read()
    if (owns() && !dirty && value !== undefined) setDraft(copy(value))
  })

  const reconcile = async (intent: number, tail: Promise<void>) => {
    await tail
    if (!owns() || intent !== revision || publication !== tail) return
    if (failedRevision === intent) {
      setBusy(false)
      setError(true)
      return
    }
    try {
      // This read must begin after the last captured save settles, rather
      // than reuse an older in-flight settings query's response.
      const value = await options.refresh(
        () => owns() && intent === revision && publication === tail
      )
      if (
        !owns() ||
        intent !== revision ||
        publication !== tail ||
        value === undefined
      )
        return
      dirty = false
      pendingDraft = undefined
      setDraft(copy(value))
      setBusy(false)
      setError(false)
    } catch {
      if (!owns() || intent !== revision || publication !== tail) return
      setBusy(false)
      setError(true)
    }
  }

  const publish = (value: Resolution) => {
    if (!owns()) return
    const snapshot = copy(value)
    const intent = ++revision
    dirty = true
    pendingDraft = copy(snapshot)
    failedRevision = undefined
    setDraft(copy(snapshot))
    setBusy(true)
    setError(false)
    const save = publication.then(() => options.save(snapshot))
    const tail = save.then(
      () => {
        if (intent === revision) failedRevision = undefined
      },
      () => {
        if (intent === revision) failedRevision = intent
      }
    )
    publication = tail
    void reconcile(intent, tail)
  }

  if (dirty) void reconcile(revision, publication)
  return {
    draft,
    busy,
    error,
    publish,
    retry: () => {
      const value = draft()
      if (owns() && !busy() && value !== undefined) publish(value)
    },
    coordinate: (index: 0 | 1, text: string) => {
      const value = draft()
      if (value?.type !== "Custom" || !/^\d+$/.test(text)) return
      const coordinate = Number(text)
      if (!Number.isInteger(coordinate) || coordinate > 65535) return
      const tuple: [number, number] = [value.value[0], value.value[1]]
      tuple[index] = coordinate
      publish({ type: "Custom", value: tuple })
    }
  }
}
