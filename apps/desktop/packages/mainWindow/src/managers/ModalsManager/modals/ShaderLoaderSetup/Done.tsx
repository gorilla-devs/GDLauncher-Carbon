import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import type { StepProps } from "."
import { useModal } from "@/managers/ModalsManager"

const Done = (_props: StepProps) => {
  const modalsContext = useModal()

  return (
    <div class="w-130 flex h-50 flex-col items-center justify-around">
      <div class="flex flex-col items-center gap-2 text-center">
        <div class="bg-primary-500/20 flex h-16 w-16 items-center justify-center rounded-full">
          <div class="i-hugeicons:tick-02 text-primary-500 h-10 w-10" />
        </div>
        <h3 class="m-0 text-lg">
          <Trans key="content:_trn_shader_loader_done_title" />
        </h3>
        <p class="text-darkSlate-100 m-0 text-sm">
          <Trans key="content:_trn_shader_loader_done_body" />
        </p>
      </div>
      <Button rounded onClick={() => modalsContext?.closeModal()}>
        <Trans key="content:_trn_shader_loader_close" />
      </Button>
    </div>
  )
}

export default Done
