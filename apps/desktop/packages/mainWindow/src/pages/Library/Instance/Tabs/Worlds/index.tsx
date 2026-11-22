import type {
  FEWorld,
  FEWorldDatapack,
  Mod,
  Progress
} from "@gd/core_module/bindings"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { useParams } from "@solidjs/router"
import {
  Show,
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount,
  untrack
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
  // The core reads it again when a copy starts and when it ends.
  const worldCopy = rspc.createQuery(() => ({
    queryKey: ["instance.getWorldCopy", instanceId()]
  }))

  // The core refreshes names and update arrows of the watched instance only.
  const prioritizeCache = rspc.createMutation(() => ({
    mutationKey: ["instance.prioritizeInstanceCache"]
  }))
  onMount(() => {
    if (!isNaN(instanceId())) prioritizeCache.mutate(instanceId())
  })

  const openWorldFolder = rspc.createMutation(() => ({
    mutationKey: ["instance.openWorldFolder"]
  }))
  const renameWorld = rspc.createMutation(() => ({
    mutationKey: ["instance.renameWorld"]
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
  const duplicateWorld = rspc.createMutation(() => ({
    mutationKey: ["instance.duplicateWorld"]
  }))
  const cancelWorldCopy = rspc.createMutation(() => ({
    mutationKey: ["instance.cancelWorldCopy"]
  }))
  const dismissTask = rspc.createMutation(() => ({
    mutationKey: ["vtask.dismissTask"]
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
  // A renamed world, while the lists are read again: the selection names
  // the old folder until they list the new one.
  const [following, setFollowing] = createSignal<{ from: string; to: string }>()
  createEffect(
    on(
      () => params.id,
      () => {
        setSelectedName(undefined)
        setFollowing(undefined)
      },
      { defer: true }
    )
  )
  const worldList = (): FEWorld[] => worlds.data ?? []
  const byName = (name: string | undefined) =>
    worldList().find((world) => world.name === name)
  const selected = () => {
    const follow = following()
    return (
      byName(selectedName()) ??
      (follow && selectedName() === follow.from
        ? byName(follow.to)
        : undefined) ??
      worldList()[0]
    )
  }
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

  /** Throws when the core refused, so the name field stays open; the global
   *  handler shows the refusal. */
  const rename = async (world: FEWorld, name: string) => {
    // The field closes when the world cannot be read, so this is only a
    // request made in the instant before. Changes blocked reach the core,
    // which refuses them with its own message.
    if (world.identity === null) throw new Error("The world cannot be read")
    const id = instanceId()
    const folder = await renameWorld.mutateAsync({
      instance_id: id,
      world: world.name,
      identity: world.identity,
      name
    })
    if (!isCurrent(id)) return
    // The folder follows the name, so the selection follows the folder,
    // unless the player chose another world meanwhile.
    const follow =
      selected()?.name === world.name
        ? { from: world.name, to: folder }
        : undefined
    if (follow) {
      setSelectedName(follow.from)
      setFollowing(follow)
    }
    try {
      await refetchAll()
    } finally {
      if (follow && isCurrent(id) && following() === follow) {
        if (selectedName() === follow.from) setSelectedName(follow.to)
        setFollowing(undefined)
      }
    }
  }

  // Each copy task is followed once, by the instance that started it; an
  // ended one is never followed again, even while the query still names it.
  const followedCopies = new Set<number>()
  const [endedCopies, setEndedCopies] = createSignal<ReadonlySet<number>>(
    new Set()
  )
  const [copyPercent, setCopyPercent] = createSignal<{
    task: number
    percent: number
  }>()
  // A duplicate asked for and not reported by the query yet: from the click
  // until the core refuses it, or until its task is named or has ended.
  const [starting, setStarting] = createSignal<{
    id: number
    source: string
    task?: number
  }>()
  const endStarting = (task: number) => {
    if (starting()?.task === task) setStarting(undefined)
  }
  createEffect(() => {
    const task = worldCopy.data?.task
    if (task !== undefined) untrack(() => endStarting(task))
  })

  /** The copy this instance is making, until its task is seen to end. */
  const runningCopy = () => {
    const copy = worldCopy.data
    return copy && !endedCopies().has(copy.task) ? copy : undefined
  }
  /** The folder of the world being copied, from the click on. */
  const copySource = () => {
    const start = starting()
    return (
      runningCopy()?.source ??
      (start?.id === instanceId() ? start.source : undefined)
    )
  }
  const copyRunning = () => copySource() !== undefined

  const selectCopy = (
    name: string,
    before: ReadonlySet<string> | undefined
  ) => {
    const named = worldList().filter((world) => worldLabel(world) === name)
    const copy = before
      ? named.find((world) => !before.has(world.name))
      : named.length === 1
        ? named[0]
        : undefined
    if (copy) setSelectedName(copy.name)
  }

  /** Follows a copy's task until it ends: its percent while it runs, the
   *  copy selected when it is done, its failure told once. */
  const followCopy = (
    task: number,
    id: number,
    copy: { source: string; name: string }
  ) => {
    if (followedCopies.has(task) || endedCopies().has(task)) return
    followedCopies.add(task)
    // The copy is published only at its end, so the list read now lacks it.
    const folders = () =>
      worlds.data && new Set(worlds.data.map((world) => world.name))
    let before = folders()
    let lastProgress: Progress | null = null
    // Left for another instance: followed again when the tab shows it.
    const leave = () => {
      followedCopies.delete(task)
      endStarting(task)
    }
    const end = () => {
      followedCopies.delete(task)
      endStarting(task)
      setEndedCopies((previous) => new Set(previous).add(task))
    }
    const poll = async () => {
      if (!isCurrent(id)) return leave()
      let outcome
      try {
        outcome = await ctx.client.query(["vtask.getTaskOutcome", task])
      } catch {
        /* A read failure cannot tell the copy's end: read it again. */
      }
      if (!isCurrent(id)) return leave()
      const result = resolveTaskPoll(outcome, lastProgress, false)
      lastProgress = result.nextLastProgress
      const { action } = result
      before ??= folders()
      if (action.kind === "progress")
        setCopyPercent({ task, percent: action.percent })
      else if (action.kind === "completed") {
        // Its row gives way to the copy once the list holds it. As for a
        // rename, a world the player chose meanwhile keeps the selection.
        try {
          await refetchAll()
          if (isCurrent(id) && selected()?.name === copy.source)
            selectCopy(copy.name, before)
        } finally {
          end()
        }
        return
      } else if (action.kind === "dismissed") {
        // Cancelled: nothing was kept and there is nothing to say.
        end()
        void refetchAll()
        return
      } else if (action.kind === "failed") {
        end()
        // The refusal of a real failure; an end that cannot be read (the
        // core restarted) is no addon change, so it is told as a copy.
        const failed = outcome?.type === "Failed"
        toast.error(
          (failed && action.message) ||
            t("instances:_trn_duplicate_failed", {
              name: copy.name,
              interpolation: { escapeValue: false }
            })
        )
        // Told here, so the failures panel does not list it again.
        if (failed) dismissTask.mutate(task)
        void refetchAll()
        return
      }
      const timer = setTimeout(() => {
        timers.delete(timer)
        void poll()
      }, 500)
      timers.add(timer)
    }
    void poll()
  }

  // A copy started elsewhere, or before the tab opened, is followed too.
  createEffect(() => {
    const copy = worldCopy.data
    if (copy) untrack(() => followCopy(copy.task, instanceId(), copy))
  })

  /** The copy's name within the core's 64 characters: the label is
   *  shortened, never the pattern around it, wherever a language puts it. */
  const copyName = (label: string) => {
    const named = (name: string) =>
      t("content:_trn_worlds.copy_name", {
        name,
        interpolation: { escapeValue: false }
      })
    // Counted as the core counts: by code points, so no emoji is split.
    const room = 64 - Array.from(named("")).length
    const kept = Array.from(label)
      .slice(0, Math.max(room, 0))
      .join("")
      .trimEnd()
    return named(kept).trim()
  }

  const duplicate = async (world: FEWorld) => {
    if (world.identity === null || copyRunning()) return
    const id = instanceId()
    const name = copyName(worldLabel(world))
    const start = { id, source: world.name }
    setStarting(start)
    try {
      const task = await duplicateWorld.mutateAsync({
        instance_id: id,
        world: world.name,
        identity: world.identity,
        name
      })
      if (starting() === start) {
        if (isCurrent(id) && !endedCopies().has(task)) {
          setStarting({ ...start, task })
          followCopy(task, id, { source: world.name, name })
        } else setStarting(undefined)
      }
      // Already named by the query: it holds the world from now on.
      if (worldCopy.data?.task === task) endStarting(task)
    } catch {
      if (starting() === start) setStarting(undefined)
      /* The global handler shows the refusal. */
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
        copy={
          runningCopy() && {
            source: runningCopy()!.source,
            name: runningCopy()!.name,
            percent:
              copyPercent()?.task === runningCopy()!.task
                ? copyPercent()!.percent
                : null,
            onCancel: () => cancelWorldCopy.mutate(instanceId())
          }
        }
      />
      <Show when={selected()}>
        {(world) => (
          <WorldDatapacks
            instanceId={instanceId()}
            world={world()}
            worldRow={worldRow(world())}
            blocked={blocked()}
            beingCopied={copySource() === world().name}
            copyRunning={copyRunning()}
            isPending={isPending}
            copiesWithUpdate={(pack) =>
              datapackCopies(worldList(), instanceMods.data ?? [], pack)
                .withUpdate
            }
            actions={{
              openFolder: () =>
                openWorldFolder.mutate({
                  instance_id: instanceId(),
                  world: world().name
                }),
              duplicate: () => void duplicate(world()),
              rename: (name) => rename(world(), name),
              deleteWorld: () => deleteWorld(world()),
              copyName: () => {
                void window.navigator.clipboard.writeText(worldLabel(world()))
                toast.success(t("notifications:_trn_copied_to_clipboard"))
              },
              updateWorld: (row) => {
                if (blocked() || copySource() === world().name) return
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
