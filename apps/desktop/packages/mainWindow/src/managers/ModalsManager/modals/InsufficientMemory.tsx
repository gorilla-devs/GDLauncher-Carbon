import { Trans } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { useNavigate } from "@solidjs/router"
import { createSignal, onMount } from "solid-js"
import { DialogFooter } from "../DialogFooter"
import { TextLink } from "@/components/TextLink"

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
  const neededMb = () => props.data?.needed_mb as number
  const availableMb = () => props.data?.available_mb as number

  let primary: HTMLButtonElement | undefined
  onMount(() => primary?.focus())

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      width="w-120"
      height="h-auto"
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-4">
          <div class="text-lightSlate-300">
            <Trans
              key="java:_trn_insufficient_memory_body"
              options={{
                requested: requestedMb(),
                needed: neededMb(),
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

        <DialogFooter>
          <TextLink
            class="whitespace-nowrap"
            onClick={async () => {
              // Persist the dismissal first if requested. The next launch
              // (not this one — this one passes skipMemoryCheck) reads
              // `is_memory_warning_dismissed` from the DB; awaiting here
              // closes the race where a fast relaunch could land before
              // the upsert is durable.
              if (dontShowAgain()) {
                await dismissWarningMutation.mutateAsync(true)
              }
              // The Java check comes first, so it passed or "Play anyway"
              // skipped it: asking again would loop.
              launchInstanceMutation.mutate({
                id: instanceId(),
                skipMemoryCheck: true,
                skipJavaCheck: true
              })
              modalsContext?.closeModal()
            }}
          >
            <Trans key="java:_trn_java_issue_play_anyway" />
          </TextLink>
          <Button
            ref={primary}
            class="whitespace-nowrap"
            onClick={() => {
              navigate(`/library/${instanceId()}/settings`)
              modalsContext?.closeModal()
            }}
          >
            <Trans key="java:_trn_adjust_memory" />
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default InsufficientMemory
