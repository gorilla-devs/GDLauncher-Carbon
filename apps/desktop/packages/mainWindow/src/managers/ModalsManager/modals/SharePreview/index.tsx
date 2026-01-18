import { Button, Spinner } from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { rspc } from "@/utils/rspcClient"
import { createMemo, For, Match, Show, Switch } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { FESharePreview } from "@gd/core_module/bindings"

interface Props {
  shareCode: string
}

/**
 * Format file size in human-readable format
 */
function formatFileSize(sizeKilobytes: number): string {
  if (sizeKilobytes < 1024) {
    return `${sizeKilobytes} KB`
  }
  const sizeMB = sizeKilobytes / 1024
  if (sizeMB < 1024) {
    return `${sizeMB.toFixed(1)} MB`
  }
  const sizeGB = sizeMB / 1024
  return `${sizeGB.toFixed(2)} GB`
}

/**
 * Format modloader display name
 */
function formatModloader(
  type: string | null | undefined,
  version: string | null | undefined
): string | null {
  if (!type) return null

  const typeDisplayNames: Record<string, string> = {
    forge: "Forge",
    fabric: "Fabric",
    quilt: "Quilt",
    neoforge: "NeoForge"
  }

  const displayType = typeDisplayNames[type.toLowerCase()] || type
  return version ? `${displayType} ${version}` : displayType
}

function SharePreview(props: ModalProps) {
  const data: () => Props = () => props.data

  const [t] = useTransContext()
  const modalsContext = useModal()

  const previewQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getSharePreview", data()?.shareCode || ""],
    enabled: !!data()?.shareCode
  }))

  const importMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.importInstanceShareCode"]
  }))

  const preview = createMemo(() => previewQuery.data as FESharePreview | undefined)

  const isExpired = createMemo(() => {
    const p = preview()
    if (!p) return false
    return new Date(p.expiresAt) < new Date()
  })

  const modloader = createMemo(() => {
    const p = preview()
    if (!p) return null
    return formatModloader(p.modloaderType, p.modloaderVersion)
  })

  const fileSize = createMemo(() => {
    const p = preview()
    if (!p) return null
    return formatFileSize(p.sizeKilobytes)
  })

  const handleImport = async () => {
    const shareCode = data()?.shareCode
    if (!shareCode) return

    try {
      await importMutation.mutateAsync(shareCode)
      modalsContext?.closeModal()
    } catch (error) {
      console.error("Failed to import share:", error)
    }
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-120 flex flex-col gap-4">
        <Switch>
          {/* Loading state */}
          <Match when={previewQuery.isLoading}>
            <div class="flex flex-col items-center justify-center py-8">
              <Spinner />
              <div class="text-lightSlate-400 mt-4 text-sm">
                <Trans key="instances:_trn_share_preview.loading" />
              </div>
            </div>
          </Match>

          {/* Error state */}
          <Match when={previewQuery.isError}>
            <div class="flex flex-col items-center justify-center py-8 text-center">
              <div class="text-red-400 mb-2 text-lg">
                <Trans key="instances:_trn_share_preview.not_found_title" />
              </div>
              <div class="text-lightSlate-500 text-sm">
                <Trans key="instances:_trn_share_preview.not_found_description" />
              </div>
            </div>
          </Match>

          {/* Preview content */}
          <Match when={preview()}>
            <div class="text-lightSlate-50">
              {/* Background image */}
              <Show when={preview()?.backgroundUrl}>
                <div class="mb-4 overflow-hidden rounded-lg">
                  <img
                    src={preview()!.backgroundUrl!}
                    alt={preview()?.title || "Instance preview"}
                    class="h-40 w-full object-cover"
                  />
                </div>
              </Show>

              {/* Title */}
              <h2 class="mb-2 text-xl font-semibold">
                {preview()?.title || t("instances:_trn_share_preview.untitled")}
              </h2>

              {/* Expired banner */}
              <Show when={isExpired()}>
                <div class="bg-red-500/20 border-red-500/30 text-red-300 mb-4 rounded-lg border p-3 text-center text-sm">
                  <Trans key="instances:_trn_share_preview.expired" />
                </div>
              </Show>

              {/* Metadata grid */}
              <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
                <Show when={preview()?.minecraftVersion}>
                  <div class="bg-darkSlate-700 rounded-lg p-3 text-center">
                    <div class="text-lightSlate-500 text-xs uppercase tracking-wide">
                      <Trans key="instances:_trn_share_preview.mc_version" />
                    </div>
                    <div class="mt-1 font-semibold">
                      {preview()!.minecraftVersion}
                    </div>
                  </div>
                </Show>

                <Show when={modloader()}>
                  <div class="bg-darkSlate-700 rounded-lg p-3 text-center">
                    <div class="text-lightSlate-500 text-xs uppercase tracking-wide">
                      <Trans key="instances:_trn_share_preview.modloader" />
                    </div>
                    <div class="mt-1 font-semibold">{modloader()}</div>
                  </div>
                </Show>

                <div class="bg-darkSlate-700 rounded-lg p-3 text-center">
                  <div class="text-lightSlate-500 text-xs uppercase tracking-wide">
                    <Trans key="instances:_trn_share_preview.mods" />
                  </div>
                  <div class="mt-1 font-semibold">
                    {preview()?.mods?.length || 0}
                  </div>
                </div>

                <Show when={fileSize()}>
                  <div class="bg-darkSlate-700 rounded-lg p-3 text-center">
                    <div class="text-lightSlate-500 text-xs uppercase tracking-wide">
                      <Trans key="instances:_trn_share_preview.size" />
                    </div>
                    <div class="mt-1 font-semibold">{fileSize()}</div>
                  </div>
                </Show>
              </div>

              {/* Mods list */}
              <Show when={(preview()?.mods?.length || 0) > 0}>
                <div class="mt-4">
                  <div class="text-lightSlate-500 mb-2 text-xs uppercase tracking-wide">
                    <Trans key="instances:_trn_share_preview.mods_list" /> (
                    {preview()!.mods.length})
                  </div>
                  <div class="bg-darkSlate-700 max-h-48 overflow-y-auto rounded-lg">
                    <For each={preview()!.mods}>
                      {(mod) => (
                        <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2 last:border-0">
                          <span class="flex-1 truncate text-sm">{mod.name}</span>
                          <div class="ml-2 flex gap-2">
                            <Show when={mod.curseforgeSlug}>
                              <a
                                href={`https://www.curseforge.com/minecraft/mc-mods/${mod.curseforgeSlug}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                class="text-orange-400 hover:text-orange-300"
                                title="View on CurseForge"
                              >
                                <div class="i-ri:fire-line text-lg" />
                              </a>
                            </Show>
                            <Show when={mod.modrinthSlug}>
                              <a
                                href={`https://modrinth.com/mod/${mod.modrinthSlug}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                class="text-green-400 hover:text-green-300"
                                title="View on Modrinth"
                              >
                                <div class="i-ri:leaf-line text-lg" />
                              </a>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>

              {/* Expiration info */}
              <Show when={!isExpired() && preview()?.expiresAt}>
                <div class="text-lightSlate-500 mt-4 text-center text-sm">
                  <Trans key="instances:_trn_share_preview.expires" />{" "}
                  {new Date(preview()!.expiresAt).toLocaleDateString(undefined, {
                    year: "numeric",
                    month: "short",
                    day: "numeric"
                  })}
                </div>
              </Show>

              {/* Disclaimer */}
              <Show when={!isExpired()}>
                <div class="bg-yellow-500/10 border-yellow-500/30 text-yellow-200 mt-4 flex items-start gap-2 rounded-lg border p-3 text-xs">
                  <div class="i-ri:alert-line mt-0.5 shrink-0 text-sm text-yellow-400" />
                  <span>
                    <Trans key="instances:_trn_share_preview.disclaimer" />
                  </span>
                </div>
              </Show>
            </div>
          </Match>
        </Switch>

        {/* Action buttons */}
        <div class="flex justify-between gap-3">
          <Button type="secondary" onClick={() => modalsContext?.closeModal()}>
            <Trans key="instances:_trn_share_preview.cancel" />
          </Button>
          <Show when={preview() && !isExpired()}>
            <Button
              type="primary"
              disabled={importMutation.isPending}
              loading={importMutation.isPending}
              onClick={handleImport}
            >
              <div class="i-ri:download-line" />
              <Trans key="instances:_trn_share_preview.import" />
            </Button>
          </Show>
        </div>
      </div>
    </ModalLayout>
  )
}

export default SharePreview
