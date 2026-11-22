import type { FEWorld, FEWorldDatapack, Mod } from "@gd/core_module/bindings"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { useParams } from "@solidjs/router"
import {
  Show,
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import {
  addonOutcomeText,
  resolveTaskPoll
} from "@/components/ModDownloadButton/hooks/resolveTaskPoll"
import useInstanceData from "../../instance.data"
import { useAddonMutations } from "../Addons/hooks/useAddonMutations"
import WorldsSidebar from "./WorldsSidebar"
import WorldDatapacks from "./WorldDatapacks"
import { datapackCopies, sameDatapackProject } from "./datapackCopies"
import { toastUpdateFailures } from "./updateFailures"
import { changesBlocked } from "@/pages/Library/shared/addons/components/ChangesBlocked"
import { asksBeforeDeletingAddons } from "@/utils/addonDeletion"
import { deletionTarget } from "@/utils/deletionTarget"
import { worldLabel } from "@/utils/worldLabel"

const noop = () => {}

const Worlds = () => {
  const [t] = useTransContext()
  const params = useParams<{ id: string }>()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const ctx = rspc.useContext()
  const routeData = useInstanceData()
  const instanceId = () => parseInt(params.id, 10)

  // Its failure is shown in the list, with a retry, not as a toast.
  const worlds = rspc.createQuery(() => ({
    queryKey: ["instance.getWorlds", instanceId()],
    meta: LOCAL_ERROR_TOAST
  }))
  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()]
  }))
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  // The core refreshes names and update arrows of the watched instance only.
  const prioritizeCache = rspc.createMutation(() => ({
    mutationKey: ["instance.prioritizeInstanceCache"]
  }))
  onMount(() => {
    if (!isNaN(instanceId())) prioritizeCache.mutate(instanceId())
  })

  const openFolder = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))
  const updateWorldDatapack = rspc.createMutation(() => ({
    mutationKey: ["instance.updateWorldDatapack"]
  }))
  const updateEverywhere = rspc.createMutation(() => ({
    mutationKey: ["instance.updateDatapackEverywhere"]
  }))
  const deleteWorldDatapack = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteWorldDatapack"]
  }))

  const refetchAll = () =>
    Promise.all([worlds.refetch(), instanceMods.refetch()])

  // A world is still a row of the addon list in the core: deleting it goes
  // through the Addons tab's deletion, with its confirmation.
  const addonMutations = useAddonMutations(
    refetchAll,
    {
      optimisticToggleAddon: noop,
      optimisticDeleteAddon: noop,
      optimisticDeleteAddons: noop,
      optimisticUpdateAddon: noop,
      rollbackToServerState: noop,
      startUpdatingMod: noop,
      stopUpdatingMod: noop
    },
    noop
  )

  // Kept by name, so a refetch keeps the world shown while it exists; the
  // first world stands in when nothing is chosen or the choice is gone.
  const [selectedName, setSelectedName] = createSignal<string>()
  createEffect(
    on(
      () => params.id,
      () => setSelectedName(undefined),
      { defer: true }
    )
  )
  const worldList = (): FEWorld[] => worlds.data ?? []
  const selected = () =>
    worldList().find((world) => world.name === selectedName()) ?? worldList()[0]
  const worldRow = (world: FEWorld) =>
    (instanceMods.data ?? []).find((row) => row.id === world.addon_id)

  const blocked = () => changesBlocked(routeData.instanceDetails.data)

  const [pending, setPending] = createSignal<ReadonlySet<string>>(new Set())
  const isPending = (id: string) => pending().has(id)
  const markPending = (ids: string[], value: boolean) =>
    setPending((previous) => {
      const next = new Set(previous)
      for (const id of ids) {
        if (value) next.add(id)
        else next.delete(id)
      }
      return next
    })
  createEffect(
    on(
      () => params.id,
      () => setPending(new Set<string>()),
      { defer: true }
    )
  )

  let disposed = false
  const timers = new Set<ReturnType<typeof setTimeout>>()
  onCleanup(() => {
    disposed = true
    timers.forEach((timer) => clearTimeout(timer))
    timers.clear()
  })
  const isCurrent = (id: number) => !disposed && instanceId() === id

  /** Waits for an update task to end, and says so when it failed. */
  const waitForTask = (taskId: number, id: number) =>
    new Promise<void>((resolve) => {
      const poll = async () => {
        if (!isCurrent(id)) return resolve()
        try {
          const outcome = await ctx.client.query([
            "vtask.getTaskOutcome",
            taskId
          ])
          const { action } = resolveTaskPoll(outcome, null, false)
          if (action.kind === "failed")
            toast.error(addonOutcomeText("content:_trn_addon_update_failed"), {
              description: action.message
            })
          if (
            action.kind === "failed" ||
            action.kind === "completed" ||
            action.kind === "dismissed"
          )
            return resolve()
        } catch {
          /* A read failure cannot certify completion. */
        }
        if (!isCurrent(id)) return resolve()
        const timer = setTimeout(() => {
          timers.delete(timer)
          void poll()
        }, 500)
        timers.add(timer)
      }
      void poll()
    })

  const updateHere = async (pack: FEWorldDatapack) => {
    const id = instanceId()
    markPending([pack.id], true)
    try {
      const taskId = await updateWorldDatapack.mutateAsync({
        instance_id: id,
        id: pack.id
      })
      await waitForTask(taskId, id)
    } catch {
      /* The global handler shows the refusal. */
    } finally {
      if (isCurrent(id)) {
        markPending([pack.id], false)
        void refetchAll()
      }
    }
  }

  const updateEverywhereFrom = async (pack: FEWorldDatapack) => {
    const id = instanceId()
    // Every world copy that is about to be updated shows as pending.
    const copies = worldList()
      .flatMap((world) => world.datapacks)
      .filter(
        (copy) =>
          copy.id === pack.id ||
          (copy.has_update && sameDatapackProject(copy, pack))
      )
      .map((copy) => copy.id)
    markPending(copies, true)
    try {
      const results = await updateEverywhere.mutateAsync({
        instance_id: id,
        origin: { World: pack.id }
      })
      toastUpdateFailures(results, worldList())
    } catch {
      /* The global handler shows the refusal. */
    } finally {
      if (isCurrent(id)) {
        markPending(copies, false)
        void refetchAll()
      }
    }
  }

  const remove = (pack: FEWorldDatapack) => {
    // As for any addon: only a pack that would be gone for good asks first.
    // A folder is the user's own, and no platform holds an unknown zip.
    const question = [
      {
        name: pack.filename,
        downloadable: !!(pack.curseforge || pack.modrinth) && !pack.is_folder
      }
    ]
    const where = deletionTarget(settings.data?.deletionThroughRecycleBin)
    if (asksBeforeDeletingAddons(question, where)) {
      modalsContext?.openModal(
        { name: "confirmAddonDeletion" },
        { addons: question, onConfirm: () => void performRemove(pack) }
      )
      return
    }
    void performRemove(pack)
  }

  const performRemove = async (pack: FEWorldDatapack) => {
    const id = instanceId()
    markPending([pack.id], true)
    try {
      await deleteWorldDatapack.mutateAsync({ instance_id: id, id: pack.id })
    } catch {
      /* The global handler shows the refusal. */
    } finally {
      if (isCurrent(id)) {
        markPending([pack.id], false)
        void refetchAll()
      }
    }
  }

  const worldParam = (world: FEWorld) => encodeURIComponent(world.name)

  /** Opens the versions of what `source` names on its platform, asking
   *  which one when both know it. With `world`, a version picked there goes
   *  into that world. */
  const openVersions = (
    name: string,
    source: Pick<Mod, "curseforge" | "modrinth">,
    world?: FEWorld
  ) => {
    const go = (platform: "curseforge" | "modrinth") => {
      const project =
        platform === "curseforge"
          ? source.curseforge!.project_id
          : source.modrinth!.project_id
      const into = world ? `&world=${worldParam(world)}` : ""
      navigator.navigate(
        `/addon/${project}/${platform}/versions?instanceId=${params.id}${into}`
      )
    }
    if (source.curseforge && source.modrinth) {
      modalsContext?.openModal(
        { name: "platformSelection" },
        { modName: name, onSelectPlatform: go }
      )
    } else if (source.curseforge) go("curseforge")
    else if (source.modrinth) go("modrinth")
  }

  const switchVersion = (world: FEWorld, pack: FEWorldDatapack) =>
    openVersions(pack.name ?? pack.filename, pack, world)

  const deleteWorld = (world: FEWorld) => {
    const row = worldRow(world)
    const target: Mod = row ?? {
      id: world.addon_id,
      filename: world.name,
      enabled: true,
      addon_type: "worlds",
      metadata: null,
      curseforge: null,
      modrinth: null,
      has_update: false,
      is_duplicate: false,
      file_size: 0
    }
    void addonMutations.handleDeleteMod(target, worldLabel(world))
  }

  const openOnPlatform = (
    world: FEWorld,
    platform: "curseforge" | "modrinth",
    where: "app" | "browser"
  ) => {
    const row = worldRow(world)
    if (platform === "curseforge" && row?.curseforge) {
      if (where === "app")
        navigator.navigate(`/addon/${row.curseforge.project_id}/curseforge`)
      else
        window.open(
          `https://www.curseforge.com/minecraft/worlds/${row.curseforge.urlslug}`,
          "_blank"
        )
    } else if (platform === "modrinth" && row?.modrinth) {
      if (where === "app")
        navigator.navigate(`/addon/${row.modrinth.project_id}/modrinth`)
      else
        window.open(
          `https://modrinth.com/project/${row.modrinth.project_id}`,
          "_blank"
        )
    }
  }

  return (
    <div class="border-darkSlate-600 border-t-solid flex h-full w-full overflow-hidden border">
      <WorldsSidebar
        worlds={worldList()}
        selected={selected()?.name}
        onSelect={setSelectedName}
        isLoading={worlds.isLoading}
        isError={worlds.isError && !worlds.data}
        onRetry={() => void worlds.refetch()}
      />
      <Show when={selected()}>
        {(world) => (
          <WorldDatapacks
            instanceId={instanceId()}
            world={world()}
            worldRow={worldRow(world())}
            blocked={blocked()}
            isPending={isPending}
            copiesWithUpdate={(pack) =>
              datapackCopies(worldList(), instanceMods.data ?? [], pack)
                .withUpdate
            }
            actions={{
              openFolder: () =>
                openFolder.mutate({
                  instance_id: instanceId(),
                  folder: "Saves"
                }),
              deleteWorld: () => deleteWorld(world()),
              copyName: () => {
                void window.navigator.clipboard.writeText(worldLabel(world()))
                toast.success(t("notifications:_trn_copied_to_clipboard"))
              },
              updateWorld: (row) => {
                if (blocked()) return
                // Its failure is told by the global handler; the arrow comes
                // back after the refetch.
                addonMutations.handleUpdateMod(row).catch(noop)
              },
              addDatapacks: () =>
                navigator.navigate(
                  `/search/datapack?instanceId=${params.id}&world=${worldParam(world())}`
                ),
              viewDetails: (row) =>
                modalsContext?.openModal(
                  { name: "modDetails" },
                  { mod: row, instanceId: instanceId() }
                ),
              // Another version of a downloaded world is a save of its own.
              switchWorldVersion: (row) =>
                openVersions(row.metadata?.name || row.filename, row),
              openInApp: (platform) => openOnPlatform(world(), platform, "app"),
              openInBrowser: (platform) =>
                openOnPlatform(world(), platform, "browser"),
              updateHere: (pack) => void updateHere(pack),
              updateEverywhere: (pack) => void updateEverywhereFrom(pack),
              switchVersion: (pack) => switchVersion(world(), pack),
              remove: (pack) => remove(pack)
            }}
          />
        )}
      </Show>
    </div>
  )
}

export default Worlds
