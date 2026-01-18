import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Input,
  Progress,
  toast
} from "@gd/ui"
import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import { rspc } from "@/utils/rspcClient"
import {
  createEffect,
  createSignal,
  For,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { FEPaginatedShares, FEShareInfo } from "@gd/core_module/bindings"
import { Trans, useTransContext } from "@gd/i18n"
import { createInfiniteQuery } from "@tanstack/solid-query"
import { formatDownloadCount } from "@/utils/helpers"
import { MAX_DOWNLOADS_LIMIT, validateMaxDownloads } from "@/utils/validation"

const PAGE_SIZE = 20

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

// Map error codes to translation keys
type ShareErrorKey =
  | "instances:_trn_share_errors.share_not_found"
  | "instances:_trn_share_errors.quota_exceeded"
  | "instances:_trn_share_errors.max_downloads_exceeded"
  | "instances:_trn_share_errors.too_many_shares"
  | "instances:_trn_share_errors.not_verified"
  | "instances:_trn_share_errors.network_error"
  | "instances:_trn_my_shares.delete_failed"
  | "instances:_trn_my_shares.regenerate_failed"
  | "instances:_trn_my_shares.update_failed"

const getShareErrorKey = <T extends ShareErrorKey>(
  code: string | null,
  fallbackKey: T
): ShareErrorKey => {
  switch (code) {
    case "SHARE_NOT_FOUND":
      return "instances:_trn_share_errors.share_not_found"
    case "QUOTA_EXCEEDED":
      return "instances:_trn_share_errors.quota_exceeded"
    case "MAX_DOWNLOADS_EXCEEDED":
      return "instances:_trn_share_errors.max_downloads_exceeded"
    case "TOO_MANY_ACTIVE_SHARES":
      return "instances:_trn_share_errors.too_many_shares"
    case "USER_NOT_VERIFIED":
      return "instances:_trn_share_errors.not_verified"
    case "NETWORK_ERROR":
      return "instances:_trn_share_errors.network_error"
    default:
      return fallbackKey
  }
}

function MyShares(props: ModalProps) {
  const [t] = useTransContext()
  const rspcContext = rspc.useContext()
  const [copySuccess, setCopySuccess] = createSignal<string | null>(null)

  // Edit state
  const [editingShare, setEditingShare] = createSignal<string | null>(null)
  const [editTitle, setEditTitle] = createSignal("")
  const [editMaxDownloads, setEditMaxDownloads] = createSignal("")

  // Quota query
  const quotaQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getUserQuota"]
  }))

  // Infinite query for shares
  const sharesQuery = createInfiniteQuery(() => ({
    queryKey: ["instance.getUserShares"],
    queryFn: (ctx) => {
      return rspcContext.client.query([
        "instance.getUserShares",
        {
          limit: PAGE_SIZE,
          offset: ctx.pageParam
        }
      ])
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage: FEPaginatedShares) => {
      const nextOffset = lastPage.offset + lastPage.items.length
      return nextOffset < lastPage.totalCount ? nextOffset : null
    }
  }))

  // Mutations
  const deleteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteShare"]
  }))

  const updateMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateShare"]
  }))

  const regenerateMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.regenerateShareCode"]
  }))

  const handleDelete = async (shareCode: string) => {
    try {
      await deleteMutation.mutateAsync(shareCode)
      toast.success(t("instances:_trn_my_shares.deleted"))
      sharesQuery.refetch()
      quotaQuery.refetch()
    } catch (err) {
      const errorCode = getErrorCode(err)
      toast.error(
        t(getShareErrorKey(errorCode, "instances:_trn_my_shares.delete_failed"))
      )
    }
  }

  const handleRegenerateCode = async (shareCode: string) => {
    try {
      const result = await regenerateMutation.mutateAsync(shareCode)
      toast.success(t("instances:_trn_my_shares.regenerated"))
      await copyToClipboard(result.newShareCode)
      sharesQuery.refetch()
    } catch (err) {
      const errorCode = getErrorCode(err)
      toast.error(
        t(
          getShareErrorKey(
            errorCode,
            "instances:_trn_my_shares.regenerate_failed"
          )
        )
      )
    }
  }

  const openEditDialog = (share: FEShareInfo) => {
    setEditingShare(share.shareCode)
    setEditTitle(share.title || "")
    setEditMaxDownloads(share.maxDownloads?.toString() || "")
  }

  const closeEditDialog = () => {
    setEditingShare(null)
    setEditTitle("")
    setEditMaxDownloads("")
  }

  const handleSaveEdit = async () => {
    const shareCode = editingShare()
    if (!shareCode) return

    try {
      const maxDownloadsValue = editMaxDownloads().trim()
      const maxDownloads = maxDownloadsValue
        ? parseInt(maxDownloadsValue) >= 1
          ? parseInt(maxDownloadsValue)
          : null
        : null

      await updateMutation.mutateAsync({
        shareCode,
        title: editTitle().trim() || null,
        maxDownloads: maxDownloads
      })
      toast.success(t("instances:_trn_my_shares.updated"))
      closeEditDialog()
      sharesQuery.refetch()
    } catch (err) {
      const errorCode = getErrorCode(err)
      toast.error(
        t(getShareErrorKey(errorCode, "instances:_trn_my_shares.update_failed"))
      )
    }
  }

  const copyToClipboard = async (shareCode: string) => {
    await navigator.clipboard.writeText(shareCode)
    setCopySuccess(shareCode)
    setTimeout(() => setCopySuccess(null), 2000)
  }

  // Flatten pages into single array
  const allShares = () => {
    if (!sharesQuery.data) return []
    return sharesQuery.data.pages.flatMap((page) => page.items)
  }

  const totalCount = () => {
    if (!sharesQuery.data || sharesQuery.data.pages.length === 0) return 0
    return sharesQuery.data.pages[0].totalCount
  }

  // Infinite scroll observer
  let scrollContainerRef: HTMLDivElement | undefined
  let loadMoreRef: HTMLDivElement | undefined

  createEffect(() => {
    // Track sharesQuery.data to re-run when data loads
    const data = sharesQuery.data
    if (!data) return

    if (!loadMoreRef || !scrollContainerRef) return

    const observer = new IntersectionObserver(
      (entries) => {
        if (
          entries[0].isIntersecting &&
          sharesQuery.hasNextPage &&
          !sharesQuery.isFetchingNextPage
        ) {
          sharesQuery.fetchNextPage()
        }
      },
      {
        root: scrollContainerRef,
        threshold: 0.1
      }
    )

    observer.observe(loadMoreRef)

    onCleanup(() => observer.disconnect())
  })

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString()
  }

  const formatSize = (sizeKb: number) => {
    if (sizeKb < 1024) return `${sizeKb} KB`
    if (sizeKb < 1024 * 1024) return `${(sizeKb / 1024).toFixed(1)} MB`
    return `${(sizeKb / 1024 / 1024).toFixed(2)} GB`
  }

  const gridCols = "grid-cols-[1fr_90px_90px_70px_90px_36px]"

  const SkeletonRow = () => (
    <div
      class={`grid ${gridCols} border-darkSlate-700 items-center border-b px-3 py-2 text-sm`}
    >
      <div class="bg-darkSlate-600 h-5 w-3/4 animate-pulse rounded" />
      <div class="bg-darkSlate-600 h-4 w-16 animate-pulse rounded" />
      <div class="bg-darkSlate-600 h-5 w-16 animate-pulse rounded" />
      <div class="bg-darkSlate-600 h-5 w-8 animate-pulse rounded text-right ml-auto" />
      <div class="bg-darkSlate-600 h-5 w-12 animate-pulse rounded text-right ml-auto" />
      <div class="flex justify-end">
        <div class="bg-darkSlate-600 h-7 w-7 animate-pulse rounded" />
      </div>
    </div>
  )

  const TableHeader = () => (
    <div
      class={`grid ${gridCols} border-darkSlate-600 border-b px-3 py-2 text-xs font-medium text-lightSlate-500`}
    >
      <div>
        <Trans key="instances:_trn_my_shares.name" />
      </div>
      <div>
        <Trans key="instances:_trn_my_shares.code" />
      </div>
      <div>
        <Trans key="instances:_trn_my_shares.expires" />
      </div>
      <div class="text-right">
        <Trans key="instances:_trn_my_shares.downloads" />
      </div>
      <div class="text-right">
        <Trans key="instances:_trn_my_shares.size" />
      </div>
      <div />
    </div>
  )

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-160 h-120 flex flex-col">
        {/* Edit Dialog Overlay */}
        <Show when={editingShare()}>
          <div class="bg-darkSlate-900/80 absolute inset-0 z-10 flex items-center justify-center">
            <div class="bg-darkSlate-800 w-80 rounded-lg p-4 shadow-lg">
              <h3 class="text-lightSlate-100 mb-4 text-lg font-medium">
                <Trans key="instances:_trn_my_shares.edit" />
              </h3>

              <div class="mb-3">
                <label class="text-lightSlate-400 mb-1 block text-sm">
                  <Trans key="instances:_trn_my_shares.edit_title_label" />
                </label>
                <Input
                  value={editTitle()}
                  onInput={(e) => setEditTitle(e.currentTarget.value)}
                  placeholder={t("instances:_trn_my_shares.name")}
                  inputColor="bg-darkSlate-700"
                  class="w-full"
                />
              </div>

              <div class="mb-4">
                <label class="text-lightSlate-400 mb-1 block text-sm">
                  <Trans key="instances:_trn_my_shares.edit_max_downloads_label" />
                </label>
                <Input
                  type="number"
                  min="1"
                  max={MAX_DOWNLOADS_LIMIT}
                  value={editMaxDownloads()}
                  onInput={(e) => {
                    const validated = validateMaxDownloads(
                      e.currentTarget.value
                    )
                    setEditMaxDownloads(validated)
                    e.currentTarget.value = validated
                  }}
                  placeholder={t(
                    "instances:_trn_my_shares.edit_max_downloads_placeholder"
                  )}
                  inputColor="bg-darkSlate-700"
                  class="w-full"
                />
              </div>

              <div class="flex justify-end gap-2">
                <Button type="secondary" size="small" onClick={closeEditDialog}>
                  <Trans key="instances:_trn_instance_share.cancel" />
                </Button>
                <Button
                  type="primary"
                  size="small"
                  onClick={handleSaveEdit}
                  loading={updateMutation.isPending}
                >
                  <Trans key="instances:_trn_my_shares.save" />
                </Button>
              </div>
            </div>
          </div>
        </Show>

        <Switch>
          <Match when={sharesQuery.isLoading}>
            <div class="flex flex-1 flex-col overflow-hidden">
              <TableHeader />
              <div class="flex-1 overflow-y-auto scrollbar-gutter-stable">
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
                <SkeletonRow />
              </div>
            </div>
          </Match>
          <Match when={sharesQuery.isError}>
            <div class="text-red-400 flex flex-1 items-center justify-center">
              <Trans key="instances:_trn_my_shares.load_failed" />
            </div>
          </Match>
          <Match when={allShares().length === 0}>
            <div class="text-lightSlate-400 flex flex-1 flex-col items-center justify-center">
              <div class="i-ri:share-line mb-4 text-4xl opacity-50" />
              <Trans key="instances:_trn_my_shares.empty" />
            </div>
          </Match>
          <Match when={allShares().length > 0}>
            <div class="flex flex-1 flex-col overflow-hidden">
              <TableHeader />
              {/* Table body */}
              <div
                ref={scrollContainerRef}
                class="flex-1 overflow-y-auto scrollbar-gutter-stable"
              >
                <For each={allShares()}>
                  {(share) => (
                    <div
                      class={`grid ${gridCols} hover:bg-darkSlate-800 border-darkSlate-700 items-center border-b px-3 py-2 text-sm transition-colors`}
                      classList={{
                        "opacity-50": share.isExpired
                      }}
                    >
                      {/* Name */}
                      <div class="text-lightSlate-100 min-w-0 truncate pr-2">
                        {share.title || "-"}
                      </div>

                      {/* Code */}
                      <div class="flex min-w-0 items-center gap-1">
                        <span class="text-lightSlate-400 truncate font-mono text-xs">
                          {share.shareCode}
                        </span>
                        <Show when={copySuccess() === share.shareCode}>
                          <div class="i-ri:check-line text-green-400 text-xs" />
                        </Show>
                      </div>

                      {/* Expires */}
                      <div>
                        <Show
                          when={share.isExpired}
                          fallback={
                            <span class="text-lightSlate-400">
                              {formatDate(share.expiresAt)}
                            </span>
                          }
                        >
                          <span class="bg-red-500/20 text-red-400 rounded px-1.5 py-0.5 text-xs font-medium">
                            <Trans key="instances:_trn_my_shares.expired" />
                          </span>
                        </Show>
                      </div>

                      {/* Downloads */}
                      <div class="text-lightSlate-400 text-right">
                        {formatDownloadCount(share.downloadCount)}
                        <Show when={share.maxDownloads}>
                          <span class="text-lightSlate-500">
                            {" / "}
                            {formatDownloadCount(share.maxDownloads!)}
                          </span>
                        </Show>
                      </div>

                      {/* Size */}
                      <div class="text-lightSlate-400 text-right">
                        {formatSize(share.sizeKilobytes)}
                      </div>

                      {/* Actions */}
                      <div class="flex justify-end">
                        <DropdownMenu>
                          <DropdownMenuTrigger class="hover:bg-darkSlate-600 flex h-7 w-7 items-center justify-center rounded">
                            <div class="i-ri:more-2-fill text-lightSlate-400" />
                          </DropdownMenuTrigger>
                          <DropdownMenuContent>
                            <Show when={!share.isExpired}>
                              <DropdownMenuItem
                                onSelect={() =>
                                  copyToClipboard(share.shareCode)
                                }
                              >
                                <div class="i-ri:file-copy-line mr-2" />
                                <Trans key="instances:_trn_my_shares.copy_code" />
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                onSelect={() => openEditDialog(share)}
                              >
                                <div class="i-ri:edit-line mr-2" />
                                <Trans key="instances:_trn_my_shares.edit" />
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                onSelect={() =>
                                  handleRegenerateCode(share.shareCode)
                                }
                              >
                                <div class="i-ri:refresh-line mr-2" />
                                <Trans key="instances:_trn_my_shares.regenerate_code" />
                              </DropdownMenuItem>
                            </Show>
                            <DropdownMenuItem
                              class="text-red-400 focus:text-red-300"
                              onSelect={() => handleDelete(share.shareCode)}
                            >
                              <div class="i-ri:delete-bin-line mr-2" />
                              <Trans key="instances:_trn_my_shares.delete" />
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </div>
                  )}
                </For>

                {/* Skeleton for next page - shows when more results available */}
                <Show when={sharesQuery.hasNextPage}>
                  <SkeletonRow />
                </Show>

                {/* Load more trigger */}
                <div ref={loadMoreRef} class="h-1" />
              </div>
            </div>
          </Match>
        </Switch>

        <div class="border-darkSlate-700 mt-4 flex items-center gap-4 border-t pt-4">
          <div class="flex flex-1 flex-col gap-2">
            {/* Quota display */}
            <Show when={quotaQuery.data}>
              {(quota) => (
                <div class="flex w-1/2 items-center gap-3">
                  <Progress
                    class="flex-1"
                    value={quota().usedKilobytes}
                    max={quota().totalKilobytes}
                  />
                  <span class="text-lightSlate-400 whitespace-nowrap text-sm">
                    {formatSize(quota().usedKilobytes)} /{" "}
                    {formatSize(quota().totalKilobytes)}
                  </span>
                </div>
              )}
            </Show>
            <span class="text-lightSlate-500 text-xs">
              <Trans key="instances:_trn_my_shares.quota_note" />
            </span>
          </div>
          <span class="text-lightSlate-500 whitespace-nowrap text-sm">
            {totalCount()} {totalCount() === 1 ? "share" : "shares"}
          </span>
        </div>
      </div>
    </ModalLayout>
  )
}

export default MyShares
