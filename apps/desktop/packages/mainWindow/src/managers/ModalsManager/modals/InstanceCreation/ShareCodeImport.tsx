import { Input } from "@gd/ui"
import { useModal } from "../.."
import {
  createEffect,
  createMemo,
  createSignal,
  Match,
  on,
  onCleanup,
  Switch
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { parseShareInput } from "@/utils/searchQueryParser"
import SharePreviewContent from "@/components/SharePreviewContent"

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

  return (
    <div class="flex h-[600px] w-full flex-col">
      <div class="flex min-h-0 flex-1 flex-col px-4 pt-4">
        {/* Input section */}
        <div class="relative mb-4 shrink-0 flex justify-center">
          <Input
            placeholder={
              t("instances:_trn_share_preview.input_placeholder") ||
              "Share code or gdl.gg link"
            }
            class={`w-full max-w-xs h-12 rounded-lg ${debouncedCode() ? "ring-2 ring-green-500" : shareInput() && !parsedShareCode() ? "ring-2 ring-red-500" : ""}`}
            inputClass="text-base text-center"
            inputColor="bg-darkSlate-800"
            value={shareInput()}
            onInput={(e) => {
              setShareInput(e.target.value)
            }}
          />
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

            {/* Invalid input (not a recognized format) */}
            <Match when={shareInput() && !parsedShareCode()}>
              <div class="flex h-full flex-col items-center justify-center text-center">
                <div class="text-lightSlate-400 text-sm">
                  <Trans key="instances:_trn_share_errors.share_not_found" />
                </div>
              </div>
            </Match>

            {/* Share preview (shown immediately, query debounced) */}
            <Match when={parsedShareCode()}>
              <SharePreviewContent
                shareCode={debouncedCode() || null}
                onImportSuccess={() => modalsContext?.closeModal()}
              />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  )
}

export default ShareCodeImport
