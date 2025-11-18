import { Progress, toast, Spinner } from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"

import { rspc } from "@/utils/rspcClient"
import { createSignal, onCleanup } from "solid-js"
import { Mod } from "@gd/core_module/bindings"
import { RSPCError } from "@rspc/client"

interface Props {
  instanceId: number
  mods: Mod[]
  onComplete?: () => void
}

const ModsUpdater = (props: ModalProps) => {
  const data: () => Props = () => props?.data
  const modalsContext = useModal()
  const [modsUpdated, setModsUpdated] = createSignal(0)
  const [isDestroyed, setIsDestroyed] = createSignal(false)

  onCleanup(() => {
    setIsDestroyed(true)
  })

  const updateModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateMod"],
    onError: (err) => {
      console.error(err)
      toast.error(`Error updating mod: ${err?.message}`)
    }
  }))

  const updateMods = async () => {
    for (const modId of data().mods) {
      try {
        await updateModMutation.mutateAsync({
          instance_id: data().instanceId,
          mod_id: modId.id
        })
      } catch (err) {
        console.error(err)
        toast.error(`Error updating mod: ${(err as RSPCError)?.message}`)
      } finally {
        setModsUpdated((prev) => prev + 1)
      }

      if (isDestroyed()) return
    }

    // Add a delay to ensure all backend tasks complete
    // The WebSocket invalidation will trigger the data refresh
    await new Promise((resolve) => setTimeout(resolve, 2000))

    toast.success("Mods updated successfully!")

    // Call the onComplete callback if provided
    const onComplete = data().onComplete
    if (onComplete) {
      onComplete()
    }

    modalsContext?.closeModal()
  }

  const currentModName = () => {
    const mod = data().mods[modsUpdated()]

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
