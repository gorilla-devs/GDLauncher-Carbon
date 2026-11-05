import { Progress, toast, Spinner } from "@gd/ui"
import { ModalProps, useModalStackEntry } from ".."
import ModalLayout from "../ModalLayout"
import { Trans, useTransContext } from "@gd/i18n"

import { rspc } from "@/utils/rspcClient"
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import { createSignal, onCleanup } from "solid-js"
import { Mod } from "@gd/core_module/bindings"
import { updateAddonBatch } from "@/components/ModDownloadButton/hooks/updateAddonBatch"

interface Props {
  instanceId: number
  mods: Mod[]
  onComplete?: () => void | Promise<unknown>
}

const ModsUpdater = (props: ModalProps) => {
  const [t] = useTransContext()
  const data: () => Props = () => props?.data
  const owningEntry = useModalStackEntry()
  const ctx = rspc.useContext()
  const captured = { ...data(), mods: [...data().mods] }
  const [modsUpdated, setModsUpdated] = createSignal(0)
  const [isDestroyed, setIsDestroyed] = createSignal(false)
  // Updates sent so far, and those still waiting for their outcome, oldest
  // first: several run at once.
  let started = 0
  const [running, setRunning] = createSignal<Mod[]>([])
  // One pending read per running update.
  const waiters = new Set<{
    timer: ReturnType<typeof setTimeout>
    release: () => void
  }>()
  const waitForNextRead = () =>
    new Promise<void>((resolve) => {
      const waiter = {
        release: resolve,
        timer: setTimeout(() => {
          waiters.delete(waiter)
          resolve()
        }, 500)
      }
      waiters.add(waiter)
    })

  onCleanup(() => {
    setIsDestroyed(true)
    for (const waiter of waiters) {
      clearTimeout(waiter.timer)
      waiter.release()
    }
    waiters.clear()
    // Closing the dialog stops the batch: updates already sent finish in the
    // core (a failure shows under the navbar's failed installs), the rest
    // never start. Say how many, since their update badges are the only
    // other sign.
    const notStarted = captured.mods.length - started
    if (notStarted > 0)
      toast.warning(
        t("content:_trn_mods_updater.not_started", { count: notStarted }),
        { description: t("content:_trn_mods_updater.not_started_description") }
      )
  })

  // A refused update is reported once per mod, by name, through `settled`
  // below (with the global toast keeping the core's sentence even after this
  // dialog closes), so there is no third toast here.
  const updateModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateMod"],
    onError: (err) => {
      console.error(err)
    }
  }))

  const updateMods = async () => {
    const outcome = await updateAddonBatch(
      captured.mods,
      (mod) =>
        updateModMutation.mutateAsync({
          instance_id: captured.instanceId,
          mod_id: mod.id
        }),
      (id) => ctx.client.query(["vtask.getTaskOutcome", id]),
      () => !isDestroyed(),
      (mod, error) => {
        if (isDestroyed()) return
        setRunning((list) => list.filter((item) => item !== mod))
        if (error !== undefined)
          toast.error(
            t("content:_trn_mods_updater.update_failed", {
              name: mod.filename,
              interpolation: { escapeValue: false }
            }),
            { description: error }
          )
        setModsUpdated((prev) => prev + 1)
      },
      waitForNextRead,
      {
        started: (mod) => {
          started++
          setRunning((list) => [...list, mod])
        }
      }
    )
    if (outcome.abandoned || isDestroyed()) return
    try {
      await captured.onComplete?.()
    } catch (error) {
      if (!isDestroyed())
        toast.error(t("content:_trn_mods_updater.refresh_error"), {
          description: extractErrorDisplay(error)
        })
      return
    }
    if (isDestroyed()) return
    if (outcome.failed === 0) {
      toast.success(
        captured.mods.some((mod) => mod.addon_type === "datapacks")
          ? t("content:_trn_mods_updater.success_datapacks")
          : t("content:_trn_mods_updater.success")
      )
    }
    owningEntry?.closeOwn()
  }

  // The oldest update still running.
  const currentModName = () => {
    const mod = running()[0]

    if (!mod) return ""

    return mod.curseforge?.name || mod.modrinth?.title || mod.filename
  }

  updateMods()

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-160 flex min-h-60 flex-col overflow-hidden">
        <div class="flex items-center text-xl">
          <div class="w-140">
            <Trans
              key="content:_trn_mods_updater.updating_mods_count"
              options={{
                mods: modsUpdated(),
                total: data().mods.length
              }}
            />
          </div>
          <Progress value={(modsUpdated() / data().mods.length) * 100} />
        </div>
        <div class="mt-20 flex flex-col items-center text-xl">
          <Trans
            key="content:_trn_mods_updater.updating_mod_text"
            options={{
              mod_name: currentModName()
            }}
          />
          <div class="mt-10">
            <Spinner class="h-10 w-10" />
          </div>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ModsUpdater
