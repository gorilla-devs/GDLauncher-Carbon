import {
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Spinner
} from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { rspc } from "@/utils/rspcClient"
import { Show, createEffect, createSignal, createMemo } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Modpack } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"

interface Props {
  instanceId: number
}

const ModPackVersionUpdate = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const instanceId = () => data()?.instanceId
  const [t] = useTransContext()
  const [selectedVersion, setSelectedVersion] = createSignal<string | null>(
    null
  )
  const navigator = useGDNavigate()
  const modalContext = useModal()

  const instance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()]
  }))

  const changeModpackMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.changeModpack"]
  }))

  // Pure reactive memo - no side effects
  const modpackData = createMemo(() => {
    const modpack = instance.data?.modpack?.modpack
    if (!modpack) return null

    if (modpack.type === "curseforge") {
      return {
        platform: "curseforge" as const,
        projectId: modpack.value.project_id,
        fileId: modpack.value.file_id
      }
    } else {
      return {
        platform: "modrinth" as const,
        projectId: modpack.value.project_id,
        fileId: modpack.value.version_id
      }
    }
  })

  const currentPlatform = createMemo(() => modpackData()?.platform)

  // Initialize selected version once when modpack data loads
  createEffect(() => {
    const data = modpackData()
    if (data && !selectedVersion()) {
      setSelectedVersion(data.fileId?.toString() || "")
    }
  })

  const responseCF = rspc.createQuery(() => {
    const data = modpackData()
    return {
      queryKey: [
        "modplatforms.curseforge.getModFiles",
        {
          modId: data?.projectId as number,
          query: {
            pageSize: 300
          }
        }
      ],
      enabled: data?.platform === "curseforge"
    }
  })

  const responseModrinth = rspc.createQuery(() => {
    const data = modpackData()
    return {
      queryKey: [
        "modplatforms.modrinth.getProjectVersions",
        {
          project_id: data?.projectId.toString()!
        }
      ],
      enabled: data?.platform === "modrinth"
    }
  })

  const response = createMemo(() =>
    currentPlatform() === "curseforge" ? responseCF : responseModrinth
  )

  const versions = createMemo(() => {
    const data = modpackData()
    if (!data) return []

    if (data.platform === "curseforge") {
      return (
        responseCF.data?.data.map((file) => ({
          id: file.id.toString(),
          name: file.displayName,
          isCurrent: file.id === data.fileId
        })) || []
      )
    }

    return (
      responseModrinth.data?.map((file) => ({
        id: file.id.toString(),
        name: file.name,
        isCurrent: file.id === data.fileId
      })) || []
    )
  })

  const handleUpdate = async () => {
    const version = selectedVersion()
    const data = modpackData()
    const id = instanceId()
    if (!version || !data || !id) return

    await changeModpackMutation.mutateAsync({
      instance: id,
      modpack: {
        type: data.platform,
        value:
          data.platform === "curseforge"
            ? {
                project_id: data.projectId,
                file_id: parseInt(version)
              }
            : {
                project_id: data.projectId.toString(),
                version_id: version
              }
      } as Modpack
    })

    modalContext?.closeModal()
    navigator.navigate("/library")
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noPadding={true}
    >
      <div class="w-120 flex flex-col gap-4 p-4">
        <Show when={response().isLoading || instance.isLoading}>
          <Spinner />
        </Show>
        <Show when={!response().isLoading && !instance.isLoading}>
          <Select
            value={selectedVersion()}
            onChange={(value) => value && setSelectedVersion(value)}
            options={versions().map((v) => v.id)}
            placeholder=""
            disallowEmptySelection={true}
            itemComponent={(itemProps) => {
              const version = versions().find(
                (v) => v.id === itemProps.item.rawValue
              )
              return (
                <SelectItem item={itemProps.item}>
                  <div class="flex w-full justify-between">
                    <span>{version?.name}</span>
                    <Show when={version?.isCurrent}>
                      <span class="text-green-500">
                        <Trans key="instances:_trn_version_current" />
                      </span>
                    </Show>
                  </div>
                </SelectItem>
              )
            }}
          >
            <SelectTrigger class="bg-darkSlate-800 w-full">
              <SelectValue<string>>
                {(state) => {
                  const selectedId = state.selectedOption()
                  const version = versions().find((v) => v.id === selectedId)
                  return (
                    <div class="flex w-full justify-between">
                      <span>{version?.name}</span>
                      <Show when={version?.isCurrent}>
                        <span class="text-green-500">
                          <Trans key="instances:_trn_version_current" />
                        </span>
                      </Show>
                    </div>
                  )
                }}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>

          <div class="flex justify-between">
            <Button
              type="outline"
              onClick={() => {
                modalContext?.closeModal()
              }}
            >
              {t("instances:_trn_cancel_export")}
            </Button>
            <Button
              type="primary"
              onClick={handleUpdate}
              disabled={!selectedVersion()}
            >
              {t("instances:_trn_instance_modal_instance_update")}
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ModPackVersionUpdate
