import { i18n, Trans, useTransContext } from "@gd/i18n"
import { Button, Spinner } from "@gd/ui"
import type { FEThemePreview } from "@gd/core_module/bindings"
import { createSignal, onCleanup, onMount, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import { DialogFooter } from "../DialogFooter"
import { ModalProps, useModal } from ".."
import { TextLink } from "@/components/TextLink"
import { extractErrorCode, extractErrorDisplay, rspc } from "@/utils/rspcClient"
import { themeErrorKey } from "@/themes/errors"
import { localizedText } from "@/themes/installed"
import {
  blocksClose,
  closeLabel,
  closesAfterInstall,
  previewImage,
  primaryAction,
  type InstallStep
} from "@/themes/install"

// The file is untrusted: the core inspects it first, and its text only ever
// reaches the page as text nodes.
const ThemeInstall = (props: ModalProps) => {
  const [t] = useTransContext()
  const modals = useModal()
  const path = (): string => props.data?.path ?? ""
  const [step, setStep] = createSignal<InstallStep>({ kind: "inspecting" })
  // The id the core installed, which Apply saves.
  const [installedId, setInstalledId] = createSignal("")
  // A preview that doesn't decode shows no picture rather than an empty box.
  const [imageFailed, setImageFailed] = createSignal(false)
  // A new drop replaces this dialog; late answers then leave it alone.
  let disposed = false
  onCleanup(() => (disposed = true))

  const inspectMutation = rspc.createMutation(() => ({
    mutationKey: ["themes.inspect"]
  }))
  const installMutation = rspc.createMutation(() => ({
    mutationKey: ["themes.install"]
  }))
  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const failed = (
    preview: FEThemePreview | null,
    error: unknown
  ): InstallStep => ({
    kind: "failed",
    preview,
    reason: themeErrorKey(extractErrorCode(error)),
    detail: extractErrorDisplay(error) || null
  })

  onMount(() => {
    void (async () => {
      try {
        const preview = await inspectMutation.mutateAsync({ path: path() })
        if (!disposed) setStep({ kind: "ready", preview })
      } catch (e) {
        if (!disposed) setStep(failed(null, e))
      }
    })()
  })

  const close = () => modals?.closeModal("themeInstall")

  const confirm = async () => {
    const current = step()
    if (current.kind !== "ready") return
    setStep({ kind: "installing", preview: current.preview })
    let id: string
    try {
      id = await installMutation.mutateAsync({ path: path() })
    } catch (e) {
      if (!disposed) setStep(failed(current.preview, e))
      return
    }
    if (disposed) return
    // The core's invalidation reloads the saved theme in place.
    if (closesAfterInstall(id, settings.data?.theme)) {
      close()
      return
    }
    setInstalledId(id)
    setStep({ kind: "installed", preview: current.preview })
  }

  const apply = async () => {
    if (settingsMutation.isPending) return
    try {
      await settingsMutation.mutateAsync({
        theme: { Set: `user:${installedId()}` }
      })
    } catch {
      // the global handler reports it; the dialog stays for another try
      return
    }
    if (!disposed) close()
  }

  const preview = (): FEThemePreview | null => {
    const s = step()
    return s.kind === "inspecting" ? null : s.preview
  }
  const language = () => i18n.language || "english"
  const failure = () => {
    const s = step()
    return s.kind === "failed" ? s : null
  }
  const action = () => primaryAction(step())

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      width="w-120"
      preventClose={blocksClose(step())}
    >
      <div class="flex flex-col gap-6">
        <Show when={step().kind === "inspecting"}>
          <div class="flex h-32 items-center justify-center">
            <Spinner />
          </div>
        </Show>
        <Show when={preview()}>
          {(p) => {
            const description = () =>
              localizedText(p().description ?? "", p().descriptions, language())
            return (
              <div class="flex flex-col gap-4">
                {/* max-h only bites in a short window, so the buttons stay on
                    screen with the longest name and description */}
                <Show when={!imageFailed() && previewImage(p())}>
                  {(src) => (
                    <img
                      src={src()}
                      alt=""
                      class="aspect-[16/10] max-h-[35vh] block w-full rounded-lg object-cover"
                      onError={() => setImageFailed(true)}
                    />
                  )}
                </Show>
                <div class="flex min-w-0 flex-col gap-1">
                  <div class="flex min-w-0 items-center gap-2">
                    <h3 class="text-lightSlate-50 m-0 min-w-0 break-words text-lg font-bold">
                      {localizedText(p().name, p().names, language())}
                    </h3>
                    <Show when={step().kind === "installed"}>
                      <div
                        role="img"
                        aria-label={t("settings:_trn_theme_installed")}
                        class="i-hugeicons:tick-02 h-5 w-5 shrink-0 text-green-500"
                      />
                    </Show>
                  </div>
                  <p class="text-lightSlate-700 m-0 break-words text-sm">
                    {p().author}
                  </p>
                  <Show when={description()}>
                    {(text) => (
                      <p class="text-lightSlate-300 m-0 mt-2 break-words">
                        {text()}
                      </p>
                    )}
                  </Show>
                </div>
              </div>
            )
          }}
        </Show>
        <Show when={failure()}>
          {(f) => (
            <div
              data-testid="theme-install-error"
              class="flex items-start gap-2"
            >
              <div
                aria-hidden="true"
                class="i-hugeicons:alert-diamond mt-0.5 h-5 w-5 shrink-0 text-yellow-500"
              />
              <div class="flex min-w-0 flex-col gap-1">
                <p class="text-lightSlate-50 m-0">{t(f().reason)}</p>
                <Show when={f().detail}>
                  {(detail) => (
                    <p class="text-lightSlate-700 m-0 break-words font-mono text-xs">
                      {detail()}
                    </p>
                  )}
                </Show>
              </div>
            </div>
          )}
        </Show>
        <DialogFooter>
          <TextLink
            data-testid="theme-install-close"
            disabled={blocksClose(step())}
            onClick={close}
          >
            {t(closeLabel(step()))}
          </TextLink>
          <Show when={action() === "install" || action() === "replace"}>
            <Button
              type="primary"
              data-testid="theme-install-confirm"
              loading={step().kind === "installing"}
              onClick={() => void confirm()}
            >
              {action() === "replace"
                ? t("settings:_trn_theme_replace_action")
                : t("settings:_trn_theme_install_action")}
            </Button>
          </Show>
          <Show when={action() === "apply"}>
            <Button
              type="primary"
              data-testid="theme-install-apply"
              loading={settingsMutation.isPending}
              onClick={() => void apply()}
            >
              <Trans key="settings:_trn_theme_apply_action" />
            </Button>
          </Show>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default ThemeInstall
