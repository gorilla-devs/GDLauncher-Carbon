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
import { extractErrorDisplay, rspc } from "@/utils/rspcClient"
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

  // Pure reactive memo - no side effects.
  //
  // `equals` is load-bearing, not an optimisation. `instance.getInstanceDetails`
  // is invalidated continuously while any task runs on the instance, and each
  // invalidation hands back a fresh object — so without a comparator this memo
  // allocates a new `{platform, projectId, fileId}` every time and notifies,
  // even when the pinned modpack has not changed at all. That cascades:
  // `versions` recomputes into a new array, `options` below changes identity,
  // and `Select` tears down and rebuilds its listbox items. The visible effect
  // is that the open dropdown destroys and recreates the row under the user's
  // cursor, mid-click, for as long as a task is in flight — which is exactly
  // when someone is most likely to be changing versions.
  //
  // Comparing the three fields stops the cascade at its source: an
  // invalidation that does not actually change the pinned pack does not
  // propagate. Same primitive `DragContext.tsx` uses for the same reason.
  const modpackData = createMemo(
    () => {
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
    },
    undefined,
    {
      equals: (prev, next) =>
        prev?.platform === next?.platform &&
        prev?.projectId === next?.projectId &&
        prev?.fileId === next?.fileId
    }
  )

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

  // The `options` array `Select` receives, with an identity that only changes
  // when the ids themselves do.
  //
  // Load-bearing for the same reason `modpackData`'s comparator is. Inlining
  // `versions().map(v => v.id)` as the prop allocates a new array every time
  // `versions` recomputes — and `versions` recomputes on every refetch of
  // `getModFiles`/`getProjectVersions`, including refetches that return byte
  // for byte what was already on screen. `Select` keys its listbox off that
  // identity, so it tore items down and rebuilt them under an open dropdown,
  // destroying the row the user's cursor was over mid-click. The `isLoading`
  // guard below does not cover this: `isLoading` is only true for the *first*
  // load, so a refetch leaves the Select mounted and swaps its options out
  // underneath.
  const versionIds = createMemo(() => versions().map((v) => v.id), undefined, {
    equals: (prev, next) =>
      prev.length === next.length && prev.every((id, i) => id === next[i])
  })

  const [updateError, setUpdateError] = createSignal<string | null>(null)

  const handleUpdate = async () => {
    const version = selectedVersion()
    const data = modpackData()
    const id = instanceId()
    if (!version || !data || !id) return

    setUpdateError(null)
    try {
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
    } catch (e) {
      setUpdateError(extractErrorDisplay(e))
      return
    }

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
            options={versionIds()}
            placeholder=""
            disallowEmptySelection={true}
            itemComponent={(itemProps) => {
              const version = versions().find(
                (v) => v.id === itemProps.item.rawValue
              )
              return (
                <SelectItem item={itemProps.item}>
                  <div
                    class="flex w-full justify-between"
                    data-testid="modpack-version-option"
                    data-version-id={itemProps.item.rawValue}
                  >
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
            <SelectTrigger
              class="bg-darkSlate-800 w-full"
              data-testid="modpack-version-select"
            >
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

          <Show when={updateError()}>
            <div
              data-testid="modpack-version-update-error"
              class="mt-2 rounded-lg border border-red-600/30 bg-red-900/20 p-3 text-sm text-red-300"
            >
              <div class="font-semibold">
                <Trans key="instances:_trn_change_version_failed" />
              </div>
              <div class="mt-1 max-h-40 overflow-y-auto break-words">
                {updateError()}
              </div>
            </div>
          </Show>

          <div class="flex justify-between">
            <Button
              type="secondary"
              onClick={() => {
                modalContext?.closeModal()
              }}
            >
              {t("instances:_trn_cancel_export")}
            </Button>
            <Button
              type="primary"
              data-testid="modpack-version-update-confirm"
              onClick={handleUpdate}
              disabled={!selectedVersion() || changeModpackMutation.isPending}
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
