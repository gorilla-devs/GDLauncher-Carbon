import { createSignal, createMemo, Show } from "solid-js"
import {
  Button,
  Input,
  Slider,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"

const ServerCreation = (props: ModalProps) => {
  const modalsContext = useModal()
  const globalStore = useGlobalStore()

  const [serverName, setServerName] = createSignal("")
  const [portValue, setPortValue] = createSignal(25565)
  const [mcVersion, setMcVersion] = createSignal("")
  const [error, setError] = createSignal("")
  const [portError, setPortError] = createSignal("")

  const createServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.createServer"]
  }))

  const releaseVersions = createMemo(() => {
    const versions = globalStore.minecraftVersions.data
    if (!versions) return []
    return versions
      .filter((v) => v.type === "release")
      .map((v) => v.id)
  })

  const selectedVersion = createMemo(() => {
    if (mcVersion()) return mcVersion()
    const versions = releaseVersions()
    return versions.length > 0 ? versions[0] : ""
  })

  const validatePort = (value: number) => {
    if (isNaN(value) || value < 1 || value > 65535) {
      setPortError("Port must be between 1 and 65535")
      return false
    }
    setPortError("")
    return true
  }

  const isFormValid = createMemo(() => {
    return (
      selectedVersion().length > 0 &&
      portValue() >= 1 &&
      portValue() <= 65535
    )
  })

  const handleCreate = async () => {
    if (!validatePort(portValue())) {
      return
    }

    setError("")

    const name = serverName().trim() || "Minecraft Server"

    try {
      await createServerMutation.mutateAsync({
        name,
        gameVersion: selectedVersion(),
        port: portValue()
      })

      modalsContext?.closeModal()
    } catch (err) {
      console.error(err)
      setError("Failed to create server. Please try again.")
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-5">
        {/* Server Name */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            Server Name
          </label>
          <Input
            placeholder="Minecraft Server"
            inputColor="bg-darkSlate-800"
            value={serverName()}
            onInput={(e) => setServerName(e.currentTarget.value)}
          />
        </div>

        {/* Game Version */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            Game Version
          </label>
          <Select
            value={selectedVersion()}
            onChange={(value) => {
              if (value) setMcVersion(value)
            }}
            options={releaseVersions()}
            placeholder="Select a version"
            disabled={releaseVersions().length === 0}
            disallowEmptySelection={true}
            itemComponent={(itemProps) => (
              <SelectItem item={itemProps.item}>
                {itemProps.item.rawValue}
              </SelectItem>
            )}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) => state.selectedOption()}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>

        {/* Port */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">Port</label>
          <Input
            type="number"
            placeholder="25565"
            inputColor="bg-darkSlate-800"
            value={String(portValue())}
            onInput={(e) => {
              const val = parseInt(e.currentTarget.value, 10)
              if (!isNaN(val)) {
                setPortValue(val)
                validatePort(val)
              }
            }}
            errorMessage={portError() || undefined}
          />
        </div>

        {/* Error message */}
        <Show when={error()}>
          <div class="text-sm text-red-500">{error()}</div>
        </Show>

        {/* Actions */}
        <div class="flex justify-between pt-2">
          <Button
            type="secondary"
            disabled={createServerMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            Cancel
          </Button>
          <Button
            disabled={!isFormValid() || createServerMutation.isPending}
            loading={createServerMutation.isPending}
            onClick={handleCreate}
          >
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
              Create Server
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ServerCreation
