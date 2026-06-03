import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useLocation } from "@solidjs/router"
import { useGDNavigate } from "@/managers/NavigationManager"

const ConfirmReinstall = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const location = useLocation()
  const navigate = useGDNavigate()

  const isServer = () => !!props.data?.isServer

  // If the user triggered reinstall from inside the detail page of the
  // instance/server being reinstalled, kick them back to the library —
  // the page is about to wipe its `.setup` and re-run the install pipeline,
  // so it'd just show a broken / loading state until the task finishes.
  const navigateAwayIfInsideDetail = () => {
    const pathname = location.pathname
    const id = props.data?.id
    if (id == null) return
    const prefix = isServer() ? `/library/server/${id}` : `/library/${id}`
    if (pathname === prefix || pathname.startsWith(`${prefix}/`)) {
      navigate.navigate(isServer() ? "/library?mode=servers" : "/library")
    }
  }

  const reinstallInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.reinstallModpack"],
    onSuccess: () => {
      toast.success(t("instances:_trn_reinstall_started"))
    },
    onError: (error) => {
      toast.error(t("instances:_trn_reinstall_failed"), {
        description: error.message
      })
    }
  }))

  const reinstallServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.reinstallServer"],
    onSuccess: () => {
      toast.success(t("instances:_trn_reinstall_started"))
    },
    onError: (error) => {
      toast.error(t("instances:_trn_reinstall_failed"), {
        description: error.message
      })
    }
  }))

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-110">
      <div class="flex flex-col gap-5">
        <div class="text-lightSlate-50">
          <Trans
            key="instances:_trn_reinstall_question"
            options={{ name: props.data?.name }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_reinstall_intro_server"
              : "instances:_trn_reinstall_intro_instance"
          )}
        </p>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_reinstall_replaced_server"
              : "instances:_trn_reinstall_replaced_instance"
          )}
        </p>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_reinstall_kept_server"
              : "instances:_trn_reinstall_kept_instance"
          )}
        </p>

        <div class="flex w-full justify-between">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_reinstall_cancel")}
          </Button>
          <Button
            type="secondary"
            onClick={() => {
              modalsContext?.closeModal()
              navigateAwayIfInsideDetail()
              if (isServer()) {
                reinstallServerMutation.mutate(props?.data?.id)
              } else {
                reinstallInstanceMutation.mutate(props?.data?.id)
              }
            }}
          >
            <div class="i-hugeicons:refresh" />
            {t("instances:_trn_reinstall_confirm")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmReinstall
