import { Button, Input } from "@gd/ui"
import { useModal } from "../.."
import { rspc } from "@/utils/rspcClient"
import { createEffect, createMemo, createSignal, Match, on, onCleanup, Show, Switch } from "solid-js"
import { VList } from "@/components/VirtuaWrapper"
import { Trans, useTransContext } from "@gd/i18n"
import { FESharePreview } from "@gd/core_module/bindings"
import { parseShareInput } from "@/utils/searchQueryParser"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

// Helper to extract error code from rspc error
const getErrorCode = (error: unknown): string | null => {
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

const getShareImportErrorKey = (code: string | null): ShareErrorKey => {
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

const ShareCodeImport = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()

  const [shareInput, setShareInput] = createSignal("")
  const [debouncedCode, setDebouncedCode] = createSignal<string | null>(null)

  const parsedShareCode = createMemo(() => parseShareInput(shareInput()))

  // Debounce the share code before fetching preview
  createEffect(
    on(parsedShareCode, (code) => {
      if (!code) {
        setDebouncedCode(null)
        return
      }

      const timeout = setTimeout(() => {
        setDebouncedCode(code)
      }, 400)

      onCleanup(() => clearTimeout(timeout))
    })
  )

  const previewQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getSharePreview", debouncedCode() || ""],
    enabled: !!debouncedCode()
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

  const canImport = createMemo(() =>
    preview() && !isExpired() && !importMutation.isPending
  )

  const errorMessage = createMemo(() => {
    if (!previewQuery.isError) return null
    const errorCode = getErrorCode(previewQuery.error)
    return t(getShareImportErrorKey(errorCode))
  })

  const handleImport = async () => {
    const code = debouncedCode()
    if (!code) return

    try {
      await importMutation.mutateAsync(code)
      modalsContext?.closeModal()
    } catch (error) {
      console.error("Failed to import share:", error)
    }
  }

  return (
    <div class="flex h-[600px] w-full flex-col">
      <div class="flex min-h-0 flex-1 flex-col px-4 pt-4">
        {/* Input section */}
        <div class="relative mb-4 shrink-0 flex justify-center">
          <Input
            placeholder={t("instances:_trn_share_preview.input_placeholder") || "Share code or gdl.gg link"}
            class={`w-full max-w-xs h-12 rounded-lg ${preview() ? "ring-2 ring-green-500" : errorMessage() ? "ring-2 ring-red-500" : ""}`}
            inputClass="text-base text-center"
            inputColor="bg-darkSlate-800"
            value={shareInput()}
            onInput={(e) => {
              setShareInput(e.target.value)
            }}
          />
          <Show when={errorMessage() && !previewQuery.isLoading}>
            <span class="text-red-400 absolute left-1/2 -translate-x-1/2 mt-14 text-xs">
              {errorMessage()}
            </span>
          </Show>
        </div>

        {/* Preview content */}
        <div class="min-h-0 flex-1">
          <Switch>
            {/* Empty state */}
            <Match when={!shareInput()}>
              <div class="flex h-full flex-col items-center justify-center text-center">
                <div class="i-hugeicons:share-08 text-lightSlate-600 mb-4 text-4xl" />
                <div class="text-lightSlate-400 text-sm">
                  <Trans key="instances:_trn_share_preview.input_placeholder" />
                </div>
              </div>
            </Match>

            {/* Loading state - skeleton preview */}
            <Match when={previewQuery.isLoading || (parsedShareCode() && !debouncedCode())}>
              <div class="text-lightSlate-50 flex h-full flex-col">
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
                    {/* Mod row skeletons */}
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
                    <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                      <div class="bg-darkSlate-600 h-4 w-3/5 animate-pulse rounded" />
                      <div class="flex gap-2">
                        <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                      </div>
                    </div>
                    <div class="border-darkSlate-600 flex items-center justify-between border-b px-3 py-2">
                      <div class="bg-darkSlate-600 h-4 w-2/5 animate-pulse rounded" />
                      <div class="flex gap-2">
                        <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
                      </div>
                    </div>
                    <div class="flex items-center justify-between px-3 py-2">
                      <div class="bg-darkSlate-600 h-4 w-1/2 animate-pulse rounded" />
                      <div class="flex gap-2">
                        <div class="bg-darkSlate-600 h-4 w-4 animate-pulse rounded" />
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
              <div class="flex h-full flex-col items-center justify-center text-center">
                <div class="text-red-400 mb-2 text-lg">
                  <Trans key="instances:_trn_share_preview.not_found_title" />
                </div>
                <div class="text-lightSlate-500 text-sm">
                  <Trans key="instances:_trn_share_preview.not_found_description" />
                </div>
              </div>
            </Match>

            {/* Invalid input (not a recognized format) */}
            <Match when={shareInput() && !parsedShareCode()}>
              <div class="flex h-full flex-col items-center justify-center text-center">
                <div class="text-lightSlate-400 text-sm">
                  <Trans key="instances:_trn_share_errors.share_not_found" />
                </div>
              </div>
            </Match>

            {/* Preview content */}
            <Match when={preview()}>
              <div class="text-lightSlate-50 flex h-full flex-col">
                {/* Hero header with background image */}
                <div class="relative shrink-0 overflow-hidden rounded-lg">
                  <img
                    src={preview()?.backgroundUrl || "/assets/images/default-instance-img.png"}
                    alt={preview()?.title || "Instance preview"}
                    class="h-32 w-full object-cover"
                  />
                  {/* Gradient overlay */}
                  <div class="absolute inset-0 bg-gradient-to-t from-darkSlate-900 via-darkSlate-900/60 to-transparent" />

                  {/* Title and metadata overlay */}
                  <div class="absolute inset-x-0 bottom-0 p-3">
                    <h2 class="mb-1 text-lg font-semibold drop-shadow-lg">
                      {preview()?.title || t("instances:_trn_share_preview.untitled")}
                    </h2>
                    <div class="text-lightSlate-400 mb-2 text-xs drop-shadow-lg">
                      <Trans key="instances:_trn_share_preview.shared_by" /> <span class="text-lightSlate-200 font-medium">{preview()?.sharerDisplayName}</span>
                    </div>

                    {/* Metadata chips */}
                    <div class="flex flex-wrap gap-1.5">
                      <Show when={preview()?.minecraftVersion}>
                        <div class="bg-darkSlate-800/80 backdrop-blur-sm rounded px-2 py-0.5 text-xs">
                          <span class="text-lightSlate-400">MC </span>
                          <span class="font-medium">{preview()!.minecraftVersion}</span>
                        </div>
                      </Show>

                      <Show when={modloader()}>
                        <div class="bg-darkSlate-800/80 backdrop-blur-sm rounded px-2 py-0.5 text-xs">
                          <span class="font-medium">{modloader()}</span>
                        </div>
                      </Show>

                      <div class="bg-darkSlate-800/80 backdrop-blur-sm rounded px-2 py-0.5 text-xs">
                        <span class="font-medium">{preview()?.mods?.length || 0}</span>
                        <span class="text-lightSlate-400"> mods</span>
                      </div>

                      <Show when={fileSize()}>
                        <div class="bg-darkSlate-800/80 backdrop-blur-sm rounded px-2 py-0.5 text-xs">
                          <span class="font-medium">{fileSize()}</span>
                        </div>
                      </Show>

                      <Show when={!isExpired() && preview()?.expiresAt}>
                        <div class="bg-darkSlate-800/80 backdrop-blur-sm rounded px-2 py-0.5 text-xs">
                          <span class="text-lightSlate-400">Expires </span>
                          <span class="font-medium">
                            {new Date(preview()!.expiresAt).toLocaleDateString(undefined, {
                              month: "short",
                              day: "numeric"
                            })}
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
                  <div class="mt-3 flex min-h-0 flex-1 flex-col">
                    <div class="text-lightSlate-500 mb-1.5 shrink-0 text-xs uppercase tracking-wide">
                      <Trans key="instances:_trn_share_preview.mods_list" /> ({preview()!.mods.length})
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
              </div>
            </Match>
          </Switch>
        </div>
      </div>

      {/* Action button */}
      <div class="flex justify-end border-t border-darkSlate-600 px-4 py-4">
        <Button
          type="primary"
          disabled={!canImport()}
          loading={importMutation.isPending}
          onClick={handleImport}
        >
          <div class="i-ri:download-line" />
          <Trans key="instances:_trn_share_preview.import" />
        </Button>
      </div>
    </div>
  )
}

export default ShareCodeImport
