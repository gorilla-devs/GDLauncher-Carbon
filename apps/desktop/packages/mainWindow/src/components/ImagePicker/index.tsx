import { createSignal, Show, mergeProps } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { Popover, PopoverContent, PopoverTrigger, Spinner } from "@gd/ui"

interface ImagePickerProps {
  imageUrl: () => string | null
  onSelect: (filePath: string) => void | Promise<void>
  onDelete?: () => void | Promise<void>
  isLoading?: () => boolean
  deletable?: boolean
  confirmDelete?: boolean
  sizeClass?: string
  dialogTitle?: string
  fileExtensions?: string[]
  placeholderIcon?: string
  class?: string
}

const ImagePicker = (props: ImagePickerProps) => {
  const [t] = useTransContext()
  const [isPopoverOpen, setIsPopoverOpen] = createSignal(false)

  const merged = mergeProps(
    {
      deletable: false,
      confirmDelete: true,
      sizeClass: "h-24 w-24",
      dialogTitle: t("general:_trn_select_image"),
      fileExtensions: ["png", "jpg", "jpeg"],
      placeholderIcon: "i-hugeicons:image-upload"
    },
    props
  )

  const handleSelectFile = async () => {
    if (props.isLoading?.()) return

    const result = await window.openFileDialog({
      title: merged.dialogTitle,
      filters: [{ name: "Images", extensions: merged.fileExtensions }],
      properties: ["openFile"]
    })

    if (result.filePaths?.[0]) {
      await props.onSelect(result.filePaths[0])
    }
  }

  const handleDeleteClick = async (e: MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (!merged.confirmDelete) {
      await props.onDelete?.()
    }
    // If confirmDelete is true, the Popover handles showing the confirmation
  }

  const handleConfirmDelete = async () => {
    setIsPopoverOpen(false)
    await props.onDelete?.()
  }

  return (
    <div class="relative">
      {/* Main picker area */}
      <div
        class={`bg-darkSlate-800 group relative flex cursor-pointer items-center justify-center rounded-xl bg-cover bg-center transition-all hover:ring-2 hover:ring-darkSlate-500 ${merged.sizeClass} ${props.class || ""}`}
        style={{
          "background-image": props.imageUrl()
            ? `url("${props.imageUrl()}")`
            : undefined
        }}
        onClick={handleSelectFile}
      >
        {/* Placeholder when no image */}
        <Show when={!props.imageUrl()}>
          <div
            class={`${merged.placeholderIcon} text-lightSlate-500 h-8 w-8`}
          />
        </Show>

        {/* Hover overlay for edit */}
        <Show when={!props.isLoading?.()}>
          <div class="absolute inset-0 flex items-center justify-center rounded-xl bg-black/50 opacity-0 transition-opacity group-hover:opacity-100">
            <div class="i-hugeicons:edit-01 h-6 w-6 text-white" />
          </div>
        </Show>

        {/* Loading overlay */}
        <Show when={props.isLoading?.()}>
          <div class="absolute inset-0 flex items-center justify-center rounded-xl bg-black/70">
            <Spinner />
          </div>
        </Show>
      </div>

      {/* Delete button with confirmation popover */}
      <Show when={merged.deletable && props.imageUrl()}>
        <div class="absolute -right-3.5 -top-3.5 z-10">
          <Show
            when={merged.confirmDelete}
            fallback={
              <div class="group/delete p-1.5">
                <div
                  class="cursor-pointer rounded-full bg-darkSlate-800 p-1 opacity-0 transition-opacity group-hover/delete:opacity-100"
                  onClick={handleDeleteClick}
                >
                  <div class="i-hugeicons:delete-02 h-5 w-5 text-red-500 transition-all hover:text-red-400" />
                </div>
              </div>
            }
          >
            <Popover
              open={isPopoverOpen()}
              onOpenChange={setIsPopoverOpen}
              placement="right"
            >
              <PopoverTrigger
                as="div"
                class="group/delete p-1.5"
                onClick={(e: MouseEvent) => e.stopPropagation()}
              >
                <div class="cursor-pointer rounded-full bg-darkSlate-800 p-1 opacity-0 transition-opacity group-hover/delete:opacity-100">
                  <div class="i-hugeicons:delete-02 h-5 w-5 text-red-500 transition-all hover:text-red-400" />
                </div>
              </PopoverTrigger>
              <PopoverContent
                class="w-auto !p-2"
                hideCloseButton
                onClick={(e: MouseEvent) => e.stopPropagation()}
              >
                <div class="flex items-center gap-2 whitespace-nowrap">
                  <span class="text-sm text-lightSlate-200">
                    {t("general:_trn_delete")}?
                  </span>
                  <button
                    class="rounded p-1 text-lightSlate-500 transition-colors hover:bg-darkSlate-700 hover:text-lightSlate-100"
                    onClick={() => setIsPopoverOpen(false)}
                  >
                    <div class="i-hugeicons:cancel-01 h-5 w-5" />
                  </button>
                  <button
                    class="rounded p-1 text-red-500 transition-colors hover:bg-red-500/20 hover:text-red-400"
                    onClick={handleConfirmDelete}
                  >
                    <div class="i-hugeicons:tick-02 h-5 w-5" />
                  </button>
                </div>
              </PopoverContent>
            </Popover>
          </Show>
        </div>
      </Show>
    </div>
  )
}

export default ImagePicker
