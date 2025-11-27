import { Trans, useTransContext } from "@gd/i18n"
import { ModalProps, useModal } from "@/managers/ModalsManager"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { Match, Switch, createSignal } from "solid-js"

const AddCustomJava = (props: ModalProps) => {
  const [value, setValue] = createSignal("")
  const [validPath, setValidPath] = createSignal<boolean | null>(null)
  const [t] = useTransContext()
  const modalsContext = useModal()

  const createCustomJavaVersionMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createCustomJavaVersion"]
  }))

  const validateJavaPath = rspc.createMutation(() => ({
    mutationKey: ["java.validateCustomJavaPath"]
  }))

  return (
    <ModalLayout width="w-100" noHeader={props.noHeader} title={props?.title}>
      <div class="flex h-full w-full flex-col items-center justify-center">
        <div class="flex w-full flex-col gap-8">
          <div class="flex w-full items-center justify-between gap-4">
            <Input
              class="w-full"
              value={value()}
              inputColor="bg-darkSlate-600"
              placeholder={t("placeholders:_trn_type_custom_java_path")}
              icon={
                <Switch>
                  <Match when={validateJavaPath.isPending}>
                    <div class="i-hugeicons:loading-03 text-lightSlate-700 flex animate-spin" />
                  </Match>
                  <Match when={value()}>
                    <Switch>
                      <Match when={validPath() === true}>
                        <div class="i-hugeicons:tick-double-02 flex text-emerald-500" />
                      </Match>
                      <Match when={validPath() === false}>
                        <div class="i-hugeicons:alert-02 flex text-yellow-500" />
                      </Match>
                    </Switch>
                  </Match>
                  <Match when={true}>
                    <div />
                  </Match>
                </Switch>
              }
              onInput={(e) => setValue(e.currentTarget.value)}
              onBlur={async () => {
                if (!value()) {
                  setValidPath(null)
                  return
                }
                const res = await validateJavaPath.mutateAsync(value())
                setValidPath(res)
              }}
            />
          </div>
          <div class="flex w-full justify-end">
            <Button
              disabled={
                !validPath() ||
                validateJavaPath.isPending ||
                createCustomJavaVersionMutation.isPending
              }
              onClick={() => {
                createCustomJavaVersionMutation.mutate(value())
                toast.success(t("notifications:_trn_custom_java_added"))
                modalsContext?.closeModal()
              }}
            >
              <Trans key="general:_trn_add_custom_java_path" />
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  )
}

export default AddCustomJava
