import { Button, toast, Progress } from "@gd/ui"
import { port, rspc } from "@/utils/rspcClient"
import { createMemo, createSignal, Match, onCleanup, Show, Switch } from "solid-js"
import { VList } from "@/components/VirtuaWrapper"
import { Trans, useTransContext } from "@gd/i18n"
import { FESharePreview } from "@gd/core_module/bindings"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

// Helper to extract error code from rspc error
export const getErrorCode = (error: unknown): string | null => {
  try {
    if (
      error &&
      typeof error === "object" &&
      "message" in error &&
      typeof error.message === "string"
    ) {
      const parsed = JSON.parse(error.message)
      if (parsed?.cause && Array.isArray(parsed.cause)) {
        for (const segment of parsed.cause) {
          if (segment?.code) {
            return segment.code
          }
        }
      }
    }
  } catch {
    // ignore parse errors
  }
  return null
}

// Map error codes to translation keys for share import errors
type ShareErrorKey =
  | "instances:_trn_share_errors.share_not_found"
  | "instances:_trn_share_errors.max_downloads_exceeded"
  | "instances:_trn_share_errors.network_error"
  | "instances:_trn_share_errors.unknown"

export const getShareImportErrorKey = (code: string | null): ShareErrorKey => {
  switch (code) {
    case "SHARE_NOT_FOUND":
      return "instances:_trn_share_errors.share_not_found"
    case "MAX_DOWNLOADS_EXCEEDED":
      return "instances:_trn_share_errors.max_downloads_exceeded"
    case "NETWORK_ERROR":
      return "instances:_trn_share_errors.network_error"
    default:
      return "instances:_trn_share_errors.unknown"
  }
}

/**
 * Format file size in human-readable format
 */
export function formatFileSize(sizeKilobytes: number): string {
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
export function formatModloader(
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

interface SharePreviewContentProps {
  shareCode: string | null
  onImportSuccess?: () => void
}

const SharePreviewContent = (props: SharePreviewContentProps) => {
  const [t] = useTransContext()

  const previewQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getSharePreview", props.shareCode ?? ""],
    enabled: !!props.shareCode
  }))

  const [isImporting, setIsImporting] = createSignal(false)
  const [importProgress, setImportProgress] = createSignal(0)
  let sseStream: EventSource | null = null

  onCleanup(() => {
    if (sseStream) {
      sseStream.close()
      sseStream = null
    }
  })

  const preview = createMemo(
    () => previewQuery.data as FESharePreview | undefined
  )

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

  const canImport = createMemo(
    () => preview() && !isExpired() && !isImporting()
  )

  const errorMessage = createMemo(() => {
    if (!previewQuery.isError) return null
    const errorCode = getErrorCode(previewQuery.error)
    return t(getShareImportErrorKey(errorCode))
  })

  const handleImport = () => {
    if (!props.shareCode) return
    setIsImporting(true)
    setImportProgress(0)

    const params = new URLSearchParams({ shareCode: props.shareCode })
    sseStream = new EventSource(
      `http://127.0.0.1:${port}/instance/importShareInstance?${params.toString()}`
    )

    sseStream.onmessage = (event) => {
      const data = JSON.parse(event.data)
      if (data.progress) {
        setImportProgress(data.progress)
      }
      if (data.finished) {
        sseStream?.close()
        sseStream = null
        setIsImporting(false)
        props.onImportSuccess?.()
      }
    }

    sseStream.addEventListener("error", (event) => {
      try {
        const data = JSON.parse((event as MessageEvent).data)
        const errorCode = data?.error?.code || null
        toast.error(t(getShareImportErrorKey(errorCode)))
      } catch {
        toast.error(t(getShareImportErrorKey(null)))
      }
      sseStream?.close()
      sseStream = null
      setIsImporting(false)
    })
  }

  return (
    <div class="text-lightSlate-50 flex flex-col">
      <Switch>
        {/* Loading state - skeleton preview (also shown while debounce pending) */}
        <Match when={!props.shareCode || previewQuery.isLoading}>
          <div class="text-lightSlate-50 flex flex-col">
            {/* Hero header skeleton */}
            <div class="relative shrink-0 overflow-hidden rounded-lg">
              {/* Background shimmer */}
              <div class="bg-darkSlate-700 h-32 w-full animate-pulse" />
              {/* Gradient overlay */}
              <div class="absolute inset-0 bg-gradient-to-t from-darkSlate-900 via-darkSlate-900/60 to-transparent" />

              {/* Title and metadata skeleton overlay */}
              <div class="absolute inset-x-0 bottom-0 p-3">
                {/* Title skeleton */}
                <div class="bg-darkSlate-600 mb-2 h-6 w-48 animate-pulse rounded" />
                {/* Shared by skeleton */}
                <div class="bg-darkSlate-600 mb-2 h-3 w-32 animate-pulse rounded" />

                {/* Metadata chips skeleton */}
                <div class="flex flex-wrap gap-1.5">
                  <div class="bg-darkSlate-600/80 h-5 w-16 animate-pulse rounded" />
                  <div class="bg-darkSlate-600/80 h-5 w-24 animate-pulse rounded" />
                  <div class="bg-darkSlate-600/80 h-5 w-14 animate-pulse rounded" />
                  <div class="bg-darkSlate-600/80 h-5 w-16 animate-pulse rounded" />
                  <div class="bg-darkSlate-600/80 h-5 w-20 animate-pulse rounded" />
                </div>
              </div>
            </div>

            {/* Mods list skeleton */}
            <div class="mt-3 flex min-h-0 flex-1 flex-col">
              {/* Header skeleton */}
              <div class="bg-darkSlate-600 mb-1.5 h-3 w-28 animate-pulse rounded" />

              {/* Mods list container */}
              <div class="bg-darkSlate-700 min-h-0 flex-1 overflow-hidden rounded-lg">
                <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                  <div class="bg-darkSlate-600 h-4 w-3/4 animate-pulse rounded" />
                  <div class="flex gap-2">
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                  </div>
                </div>
                <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                  <div class="bg-darkSlate-600 h-4 w-1/2 animate-pulse rounded" />
                  <div class="flex gap-2">
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                  </div>
                </div>
                <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                  <div class="bg-darkSlate-600 h-4 w-2/3 animate-pulse rounded" />
                  <div class="flex gap-2">
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                  </div>
                </div>
                <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                  <div class="bg-darkSlate-600 h-4 w-1/3 animate-pulse rounded" />
                  <div class="flex gap-2">
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                  </div>
                </div>
                <div class="flex items-center justify-between px-3 py-2">
                  <div class="bg-darkSlate-600 h-4 w-3/5 animate-pulse rounded" />
                  <div class="flex gap-2">
                    <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                  </div>
                </div>
              </div>
            </div>

            {/* Disclaimer - static, no loading needed */}
            <div class="bg-yellow-500/10 border-yellow-500/30 text-yellow-200 mt-3 flex shrink-0 items-center gap-2 rounded-lg border px-3 py-2 text-xs">
              <div class="i-ri:alert-line shrink-0 text-yellow-400" />
              <span>
                <Trans key="instances:_trn_share_preview.disclaimer" />
              </span>
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
              {errorMessage() || (
                <Trans key="instances:_trn_share_preview.not_found_description" />
              )}
            </div>
          </div>
        </Match>

        {/* Preview content */}
        <Match when={preview()}>
          <div class="text-lightSlate-50 flex flex-col">
            {/* Hero header with background image */}
            <div class="relative shrink-0 overflow-hidden rounded-lg">
              <img
                src={
                  preview()?.backgroundUrl ||
                  "/assets/images/default-instance-img.png"
                }
                alt={preview()?.title || "Instance preview"}
                class="h-32 w-full object-cover"
              />
              {/* Gradient overlay */}
              <div class="absolute inset-0 bg-gradient-to-t from-darkSlate-900 via-darkSlate-900/60 to-transparent" />

              {/* Title and metadata overlay */}
              <div class="absolute inset-x-0 bottom-0 p-3">
                <h2 class="mb-1 text-lg font-semibold drop-shadow-lg">
                  {preview()?.title ||
                    t("instances:_trn_share_preview.untitled")}
                </h2>
                <div class="text-lightSlate-400 mb-2 text-xs drop-shadow-lg">
                  <Trans key="instances:_trn_share_preview.shared_by" />{" "}
                  <span class="text-lightSlate-200 font-medium">
                    {preview()?.sharerDisplayName}
                  </span>
                </div>

                {/* Metadata chips */}
                <div class="flex flex-wrap gap-1.5">
                  <Show when={preview()?.minecraftVersion}>
                    <div class="bg-darkSlate-800/80 rounded px-2 py-0.5 text-xs backdrop-blur-sm">
                      <span class="text-lightSlate-400">MC </span>
                      <span class="font-medium">
                        {preview()!.minecraftVersion}
                      </span>
                    </div>
                  </Show>

                  <Show when={modloader()}>
                    <div class="bg-darkSlate-800/80 rounded px-2 py-0.5 text-xs backdrop-blur-sm">
                      <span class="font-medium">{modloader()}</span>
                    </div>
                  </Show>

                  <div class="bg-darkSlate-800/80 rounded px-2 py-0.5 text-xs backdrop-blur-sm">
                    <span class="font-medium">
                      {preview()?.mods?.length || 0}
                    </span>
                    <span class="text-lightSlate-400"> mods</span>
                  </div>

                  <Show when={fileSize()}>
                    <div class="bg-darkSlate-800/80 rounded px-2 py-0.5 text-xs backdrop-blur-sm">
                      <span class="font-medium">{fileSize()}</span>
                    </div>
                  </Show>

                  <Show when={!isExpired() && preview()?.expiresAt}>
                    <div class="bg-darkSlate-800/80 rounded px-2 py-0.5 text-xs backdrop-blur-sm">
                      <span class="text-lightSlate-400">Expires </span>
                      <span class="font-medium">
                        {new Date(preview()!.expiresAt).toLocaleDateString(
                          undefined,
                          {
                            month: "short",
                            day: "numeric"
                          }
                        )}
                      </span>
                    </div>
                  </Show>
                </div>
              </div>
            </div>

            {/* Expired banner */}
            <Show when={isExpired()}>
              <div class="bg-red-500/20 border-red-500/30 text-red-300 mt-3 shrink-0 rounded-lg border p-2 text-center text-sm">
                <Trans key="instances:_trn_share_preview.expired" />
              </div>
            </Show>

            {/* Mods list - fills remaining space */}
            <Show when={(preview()?.mods?.length || 0) > 0}>
              <div class="mt-3 flex h-60 flex-col">
                <div class="text-lightSlate-500 mb-1.5 shrink-0 text-xs uppercase tracking-wide">
                  <Trans key="instances:_trn_share_preview.mods_list" /> (
                  {preview()!.mods.length})
                </div>
                <VList
                  data={preview()!.mods}
                  class="bg-darkSlate-700 min-h-0 flex-1 rounded-lg"
                >
                  {(mod) => (
                    <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-1.5">
                      <span class="flex-1 truncate text-sm">{mod.name}</span>
                      <div class="ml-2 flex gap-2">
                        <Show when={mod.curseforgeSlug}>
                          <img src={CurseforgeLogo} class="h-4 w-4" />
                        </Show>
                        <Show when={mod.modrinthSlug}>
                          <img src={ModrinthLogo} class="h-4 w-4" />
                        </Show>
                      </div>
                    </div>
                  )}
                </VList>
              </div>
            </Show>

            {/* Disclaimer - compact */}
            <Show when={!isExpired()}>
              <div class="bg-yellow-500/10 border-yellow-500/30 text-yellow-200 mt-3 flex shrink-0 items-center gap-2 rounded-lg border px-3 py-2 text-xs">
                <div class="i-ri:alert-line shrink-0 text-yellow-400" />
                <span>
                  <Trans key="instances:_trn_share_preview.disclaimer" />
                </span>
              </div>
            </Show>

            {/* Import button */}
            <div class="mt-3 flex justify-end">
              <Button
                type="primary"
                disabled={!canImport()}
                onClick={handleImport}
              >
                <Switch>
                  <Match when={isImporting()}>
                    <div class="w-20">
                      <Progress color="bg-primary-400" value={importProgress()} />
                    </div>
                  </Match>
                  <Match when={!isImporting()}>
                    <div class="i-ri:download-line" />
                    <Trans key="instances:_trn_share_preview.import" />
                  </Match>
                </Switch>
              </Button>
            </div>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default SharePreviewContent
