import type { FEWorld, Mod } from "@gd/core_module/bindings"
import { useTransContext } from "@gd/i18n"
import { toast } from "@gd/ui"
import { createSignal, onCleanup } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { extractErrorCodeCause } from "@/utils/rspcErrorText"
import { useModal } from "@/managers/ModalsManager"
import {
  datapackCopies,
  sameDatapackProject
} from "../../Worlds/datapackCopies"
import { toastUpdateFailures } from "../../Worlds/updateFailures"
import { labelOfFolder, worldLabel } from "@/utils/worldLabel"

/** A world the core can copy a data pack into. */
export type ReadableWorld = FEWorld & { identity: string }

/** The core copies `.zip` data packs only. */
export const copiesToWorld = (row: Mod) =>
  row.addon_type === "datapacks" && row.filename.toLowerCase().endsWith(".zip")

/**
 * The instance's worlds as the Addons tab's data pack rows use them: which
 * worlds hold a copy of a row's project, adding a row to a world, and
 * updating every copy of its project.
 */
export const useDatapackWorlds = (options: {
  instanceId: () => number
  rows: () => readonly Mod[]
  refetchAddons: () => Promise<unknown>
  startUpdatingMod: (_id: string) => void
  stopUpdatingMod: (_id: string) => void
}) => {
  const [t] = useTransContext()
  const modalsContext = useModal()

  // A failure only leaves the rows without world names: no toast.
  const worlds = rspc.createQuery(() => ({
    queryKey: ["instance.getWorlds", options.instanceId()],
    enabled: options.rows().some((row) => row.addon_type === "datapacks"),
    meta: LOCAL_ERROR_TOAST
  }))
  const copy = rspc.createMutation(() => ({
    mutationKey: ["instance.copyDatapack"]
  }))
  const updateEverywhere = rspc.createMutation(() => ({
    mutationKey: ["instance.updateDatapackEverywhere"]
  }))

  let disposed = false
  onCleanup(() => {
    disposed = true
  })
  const isCurrent = (id: number) => !disposed && options.instanceId() === id

  const worldList = (): readonly FEWorld[] => worlds.data ?? []
  const readableWorlds = () =>
    worldList().filter(
      (world): world is ReadableWorld => world.identity !== null
    )

  /** The worlds holding a copy of the row's project, in the list's order. */
  const holders = (row: Mod) =>
    worldList()
      .filter((world) =>
        world.datapacks.some((pack) => sameDatapackProject(pack, row))
      )
      .map(worldLabel)

  const copiesWithUpdate = (row: Mod) =>
    datapackCopies(worldList(), options.rows(), row).withUpdate

  const [copying, setCopying] = createSignal<ReadonlySet<string>>(new Set())
  const markCopying = (id: string, value: boolean) =>
    setCopying((previous) => {
      const next = new Set(previous)
      if (value) next.add(id)
      else next.delete(id)
      return next
    })
  const isCopying = (id: string) => copying().has(id)

  const addToWorld = async (row: Mod, world: ReadableWorld) => {
    const id = options.instanceId()
    const name = row.metadata?.name || row.filename
    const send = async (replace: boolean) => {
      markCopying(row.id, true)
      try {
        const destination = await copy.mutateAsync({
          instance_id: id,
          mod_id: row.id,
          world: { name: world.name, identity: world.identity },
          ...(replace && { replace: true })
        })
        toast.success(
          t("content:_trn_datapack_staging.added", {
            name,
            world: labelOfFolder(worldList(), destination),
            interpolation: { escapeValue: false }
          })
        )
        if (isCurrent(id)) void worlds.refetch()
      } catch (error) {
        // Any other refusal is shown by the global handler.
        const cause = extractErrorCodeCause(error)
        if (replace || cause?.code !== "ALREADY_IN_WORLD") return
        const filenames =
          (cause.data as { filenames?: string[] } | undefined)?.filenames ?? []
        modalsContext?.openModal(
          { name: "confirmChoice" },
          {
            title: t("content:_trn_worlds.replace_title", {
              world: worldLabel(world),
              interpolation: { escapeValue: false }
            }),
            message: t("content:_trn_worlds.replace_files", {
              files: filenames.join(", "),
              interpolation: { escapeValue: false }
            }),
            confirmLabel: t("content:_trn_worlds.replace"),
            onConfirm: () => void send(true)
          }
        )
      } finally {
        markCopying(row.id, false)
      }
    }
    await send(false)
  }

  const updateEverywhereFrom = async (row: Mod) => {
    const id = options.instanceId()
    options.startUpdatingMod(row.id)
    try {
      const results = await updateEverywhere.mutateAsync({
        instance_id: id,
        origin: { Instance: row.id }
      })
      toastUpdateFailures(results, worldList())
    } catch {
      /* The global handler shows the refusal. */
    } finally {
      if (isCurrent(id)) {
        options.stopUpdatingMod(row.id)
        void options.refetchAddons()
        void worlds.refetch()
      }
    }
  }

  return {
    worlds,
    readableWorlds,
    holders,
    copiesWithUpdate,
    isCopying,
    addToWorld,
    updateEverywhere: updateEverywhereFrom
  }
}
