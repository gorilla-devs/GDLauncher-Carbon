import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { createSignal, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."

type Unavailable = NonNullable<
  Awaited<ReturnType<typeof window.getUnavailableRuntimePath>>
>

/**
 * This start could not open the data folder, so it uses the default one. Once
 * the drive is connected again, a restart opens the data folder. A user whose
 * folder is gone for good keeps the default one instead, and is not asked again.
 */
const RuntimePathUnavailable = (props: ModalProps) => {
  const modalsContext = useModal()
  const value = () => props.data as Unavailable
  const [restarting, setRestarting] = createSignal(false)
  const [confirming, setConfirming] = createSignal(false)
  const [keeping, setKeeping] = createSignal(false)
  const [error, setError] = createSignal<string>()
  const paths = () => ({
    path: value().folder,
    defaultPath: value().current,
    // Rendered as text; escaping would show "&#x2F;" for "/".
    interpolation: { escapeValue: false }
  })

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title} width="w-180">
      <div class="text-lightSlate-300 flex flex-col gap-4">
        <Show
          when={confirming()}
          fallback={
            <>
              <p class="break-words">
                <Trans
                  key="java:_trn_runtime_recovery.unavailable"
                  options={paths()}
                />
              </p>
              <p class="break-words">
                <Trans key="java:_trn_runtime_recovery.unavailable_gone" />
              </p>
              <div class="flex w-full justify-between gap-2">
                <div class="flex gap-2">
                  <Button
                    type="secondary"
                    disabled={restarting()}
                    onClick={async () => {
                      setRestarting(true)
                      try {
                        await window.relaunch()
                      } catch (error) {
                        console.error("Could not restart GDLauncher", error)
                      } finally {
                        setRestarting(false)
                      }
                    }}
                  >
                    <Trans key="java:_trn_runtime_recovery.restart" />
                  </Button>
                  <Button
                    type="secondary"
                    disabled={restarting()}
                    onClick={() => setConfirming(true)}
                  >
                    <Trans key="java:_trn_runtime_recovery.keep_default" />
                  </Button>
                </div>
                <Button onClick={() => modalsContext?.closeModal()}>
                  <Trans key="general:_trn_ok" />
                </Button>
              </div>
            </>
          }
        >
          <p class="break-words">
            <Trans
              key="java:_trn_runtime_recovery.keep_default_confirm"
              options={paths()}
            />
          </p>
          <Show when={error()}>
            <p class="text-red-400 break-words">
              <Trans
                key="java:_trn_runtime_recovery.keep_default_failed"
                options={{
                  error: error(),
                  interpolation: { escapeValue: false }
                }}
              />
            </p>
          </Show>
          <div class="flex w-full justify-between gap-2">
            <Button
              type="secondary"
              disabled={keeping()}
              onClick={() => {
                setError(undefined)
                setConfirming(false)
              }}
            >
              <Trans key="general:_trn_back" />
            </Button>
            <Button
              disabled={keeping()}
              onClick={async () => {
                setKeeping(true)
                setError(undefined)
                try {
                  await window.keepDefaultRuntimePath()
                  modalsContext?.closeModal()
                } catch (error) {
                  console.error("Could not keep the default data folder", error)
                  setError(String(error))
                } finally {
                  setKeeping(false)
                }
              }}
            >
              <Trans key="java:_trn_runtime_recovery.keep_default" />
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default RuntimePathUnavailable
