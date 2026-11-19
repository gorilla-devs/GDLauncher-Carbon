import { createMemo, createSignal, For, Show } from "solid-js"
import { Button, Checkbox } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { remaining, splitPath } from "./files"

/** The files a modpack update kept as the player has them although the new
 * version changed them too, with the one way to take the modpack's instead.
 * Opened from the notice the update leaves; `data.files` is its list. */
const KeptPackFiles = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const instance = () => props.data?.id as number
  const [files, setFiles] = createSignal<string[]>(props.data?.files ?? [])
  // Picking files is for the player who wants to keep some; by default the
  // one button takes them all.
  const [choosing, setChoosing] = createSignal(false)
  const [skipped, setSkipped] = createSignal<ReadonlySet<string>>(new Set())
  const [failure, setFailure] = createSignal<string | null>(null)

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const replaceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.usePackVersions"],
    meta: LOCAL_ERROR_TOAST
  }))
  const locateMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.keptFileLocation"],
    meta: LOCAL_ERROR_TOAST
  }))

  const selected = createMemo(() =>
    files().filter((path) => !skipped().has(path))
  )

  const toggle = (path: string) => {
    const next = new Set(skipped())
    if (!next.delete(path)) next.add(path)
    setSkipped(next)
  }

  const showInFolder = async (path: string) => {
    try {
      window.openFolder(
        await locateMutation.mutateAsync({ instance: instance(), path })
      )
    } catch (error) {
      setFailure(extractErrorDisplay(error))
    }
  }

  const replace = async () => {
    setFailure(null)
    try {
      const results = await replaceMutation.mutateAsync({
        instance: instance(),
        paths: selected()
      })
      const failed = results.filter((result) => result.error !== null)
      if (failed.length === 0) {
        modalsContext?.closeModal()
        return
      }
      setFiles(remaining(files(), results))
      setFailure(
        t("instances:_trn_kept_files_failed", {
          count: failed.length,
          reason: failed[0].error,
          interpolation: { escapeValue: false }
        })
      )
    } catch (error) {
      setFailure(extractErrorDisplay(error))
    }
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-110">
      <div class="flex flex-col gap-4">
        <div class="text-lightSlate-50">
          <Trans
            key="instances:_trn_kept_files_intro"
            options={{ name: props.data?.name }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>

        <div class="text-lightSlate-500 flex h-6 items-center justify-between text-xs">
          <Show
            when={choosing()}
            fallback={
              <>
                <span>
                  {t("instances:_trn_kept_files_count", {
                    count: files().length
                  })}
                </span>
                <Button
                  type="text"
                  data-testid="kept-files-choose"
                  onClick={() => setChoosing(true)}
                >
                  {t("instances:_trn_kept_files_choose")}
                </Button>
              </>
            }
          >
            <Checkbox
              checked={selected().length === files().length}
              indeterminate={
                selected().length > 0 && selected().length < files().length
              }
              onChange={(all) => setSkipped(new Set(all ? [] : files()))}
            >
              <span class="text-lightSlate-300" data-testid="kept-files-all">
                {t("instances:_trn_kept_files_selected", {
                  selected: selected().length,
                  total: files().length
                })}
              </span>
            </Checkbox>
          </Show>
        </div>

        <div class="-mx-2 flex max-h-60 flex-col overflow-y-auto">
          <For each={files()}>
            {(path) => {
              const { name, folder } = splitPath(path)
              return (
                <div
                  class="hover:bg-darkSlate-600 group flex items-center gap-2.5 rounded-md px-2 py-1.5"
                  data-testid="kept-file-row"
                >
                  <Show
                    when={choosing()}
                    fallback={
                      <div class="i-hugeicons:file-01 text-lightSlate-500 h-4 w-4 shrink-0" />
                    }
                  >
                    <Checkbox
                      ariaLabel={path}
                      checked={!skipped().has(path)}
                      onChange={() => toggle(path)}
                    />
                  </Show>
                  <span
                    class="min-w-0 shrink truncate text-sm"
                    classList={{
                      "text-lightSlate-50": !skipped().has(path),
                      "text-lightSlate-500": skipped().has(path)
                    }}
                    title={path}
                  >
                    {name}
                  </span>
                  <span class="text-lightSlate-500 min-w-0 flex-1 truncate text-xs">
                    {folder}
                  </span>
                  <div
                    role="button"
                    tabIndex={0}
                    aria-label={t("instances:_trn_open_folder")}
                    title={t("instances:_trn_open_folder")}
                    data-testid="kept-file-show"
                    class="i-hugeicons:folder-open text-darkSlate-300 group-hover:text-lightSlate-300 hover:!text-lightSlate-50 h-4 w-4 shrink-0 cursor-pointer"
                    onClick={() => showInFolder(path)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") showInFolder(path)
                    }}
                  />
                </div>
              )
            }}
          </For>
        </div>

        <Show when={failure()}>
          {(message) => (
            <p
              class="m-0 text-sm text-red-400"
              data-testid="kept-files-failure"
            >
              {message()}
            </p>
          )}
        </Show>

        <Show when={settings.data?.deletionThroughRecycleBin}>
          <span class="text-lightSlate-500 text-xs">
            {t("instances:_trn_kept_files_recycle_bin")}
          </span>
        </Show>

        <div class="flex w-full justify-between">
          <Button type="secondary" onClick={() => modalsContext?.closeModal()}>
            {t("general:_trn_close")}
          </Button>
          <Button
            data-testid="kept-files-use-pack"
            disabled={selected().length === 0}
            loading={replaceMutation.isPending}
            onClick={replace}
          >
            {t("instances:_trn_kept_files_use_pack")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default KeptPackFiles
