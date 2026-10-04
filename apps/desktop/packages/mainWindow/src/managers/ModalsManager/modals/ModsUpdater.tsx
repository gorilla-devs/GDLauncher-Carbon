import { Progress, toast, Spinner } from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"

import { rspc } from "@/utils/rspcClient"
import { createSignal, onCleanup } from "solid-js"
import { Mod } from "@gd/core_module/bindings"
import { updateAddonBatch } from "@/components/ModDownloadButton/hooks/updateAddonBatch"

interface Props {
  instanceId: number
  mods: Mod[]
  onComplete?: () => void
}

const ModsUpdater = (props: ModalProps) => {
  const data: () => Props = () => props?.data
  const modalsContext = useModal()
  const ctx = rspc.useContext()
  const captured = { ...data(), mods: [...data().mods] }
  const [modsUpdated, setModsUpdated] = createSignal(0)
  const [isDestroyed, setIsDestroyed] = createSignal(false)

  onCleanup(() => {
    setIsDestroyed(true)
  })

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
        if (error !== undefined)
          toast.error(
            t("content:_trn_mods_updater.update_failed", {
              name: mod.filename,
              interpolation: { escapeValue: false }
            }),
            { description: error }
          )
        setModsUpdated((prev) => prev + 1)
      }
    )
    if (outcome.abandoned || isDestroyed()) return
    if (outcome.failed === 0) {
      toast.success(
        captured.mods.some((mod) => mod.addon_type === "datapacks")
          ? t("content:_trn_mods_updater.success_datapacks")
          : t("content:_trn_mods_updater.success")
      )
    }
    captured.onComplete?.()
    modalsContext?.closeModal()
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
