import { Button } from "@gd/ui"
import { ModalProps } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"
import BisectHostingHorizontalLogo from "@/components/BisectHostingHorizontalLogo"

const BisectHostingAffiliate = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex flex-col overflow-hidden">
        <div class="flex justify-center">
          <div class="w-50 flex justify-center">
            <BisectHostingHorizontalLogo />
          </div>
        </div>

        <p>
          <Trans key="modals.bisectHosting.promotion">
            {" "}
            <span class="text-brands-bisecthosting">{""}</span>
            {""}
            <span class="text-brands-bisecthosting">{""}</span>
            {""}
            <span class="text-brands-bisecthosting">{""}</span>
            {""}
          </Trans>
        </p>
        <div class="mb-4 mt-6 flex flex-1 items-center justify-center">
          <Button
            onClick={() => {
              window.open("https://bisecthosting.gdlauncher.com", "_blank")
            }}
          >
            <Trans key="modals.bisectHosting.buttonText" />
            <div class="i-ri:external-link-fill" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default BisectHostingAffiliate
