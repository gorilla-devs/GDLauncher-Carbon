import { Trans } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { useNavigate } from "@solidjs/router"
import { createSignal } from "solid-js"

function InsufficientMemory(props: ModalProps) {
  const navigate = useNavigate()
  const modalsContext = useModal()
  const [dontShowAgain, setDontShowAgain] = createSignal(false)

  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  const dismissWarningMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setMemoryWarningDismissed"]
  }))

  const instanceId = () => props.data?.instance_id as number
  const requestedMb = () => props.data?.requested_mb as number
  const availableMb = () => props.data?.available_mb as number

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-120">
      <div class="flex min-h-50 flex-col justify-between gap-6 overflow-hidden">
        <div class="flex flex-col gap-4">
          <div class="flex items-center gap-2 text-xl font-bold text-yellow-400">
            <div class="i-hugeicons:alert-02 h-6 w-6 shrink-0" />
            <Trans key="java:_trn_insufficient_memory_title" />
          </div>
          <div class="text-lightSlate-300">
            <Trans
              key="java:_trn_insufficient_memory_body"
              options={{
                requested: requestedMb(),
                available: availableMb()
              }}
            />
          </div>
          <Checkbox
            checked={dontShowAgain()}
            onChange={(checked) => setDontShowAgain(checked)}
          >
            <span class="text-lightSlate-300 text-sm">
              <Trans key="java:_trn_insufficient_memory_dont_show_again" />
            </span>
          </Checkbox>
        </div>

        <div class="flex w-full justify-between">
          <Button
            type="secondary"
            size="large"
            onClick={async () => {
              // Persist the dismissal first if requested. The next launch
              // (not this one — this one passes skipMemoryCheck) reads
              // `is_memory_warning_dismissed` from the DB; awaiting here
              // closes the race where a fast relaunch could land before
              // the upsert is durable.
              if (dontShowAgain()) {
                await dismissWarningMutation.mutateAsync(true)
              }
              launchInstanceMutation.mutate({
                id: instanceId(),
                skipMemoryCheck: true
              })
              modalsContext?.closeModal()
            }}
          >
            <div class="flex items-center gap-2">
              <i class="i-hugeicons:play h-4 w-4" />
              <Trans key="java:_trn_launch_anyway" />
            </div>
          </Button>
          <Button
            type="primary"
            size="large"
            onClick={() => {
              navigate(`/library/${instanceId()}/settings`)
              modalsContext?.closeModal()
            }}
          >
            <div class="flex items-center gap-2">
              <i class="i-hugeicons:settings-02 h-4 w-4" />
              <Trans key="java:_trn_adjust_memory" />
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default InsufficientMemory
