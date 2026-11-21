import { ModalProps, useModal } from ".."
import { CancelLink, DialogFooter } from "../DialogFooter"
import ModalLayout from "../ModalLayout"
import { Trans, useTransContext } from "@gd/i18n"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { FEUnifiedPlatform } from "@gd/core_module/bindings"
import { PlatformLogo } from "@/components/PlatformLogo"

interface Props {
  onSelectPlatform: (platform: FEUnifiedPlatform) => void
  modName: string
}

const PlatformSelection = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const modalContext = useModal()
  const [t] = useTransContext()

  const handleSelectPlatform = (platform: FEUnifiedPlatform) => {
    data().onSelectPlatform(platform)
    modalContext?.closeModal()
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("instances:_trn_select_platform")}
      noPadding={false}
    >
      <div class="flex w-96 flex-col gap-4">
        <p class="text-lightSlate-400">
          <Trans
            key="instances:_trn_select_platform_description"
            options={{
              modName: data().modName
            }}
          />
        </p>

        <div class="mt-4 flex flex-col gap-3">
          <button
            class="bg-darkSlate-700 hover:bg-darkSlate-600 border-darkSlate-500 flex items-center gap-4 rounded-lg border p-4 transition-colors"
            onClick={() => handleSelectPlatform("curseforge")}
          >
            <PlatformLogo
              platform="curseforge"
              class="h-8 w-8"
              alt="CurseForge"
            />
            <div class="flex flex-col items-start">
              <span class="font-medium">{t("enums:_trn_curseforge")}</span>
              <span class="text-lightSlate-500 text-sm">
                {t("instances:_trn_view_versions_on_curseforge")}
              </span>
            </div>
          </button>

          <button
            class="bg-darkSlate-700 hover:bg-darkSlate-600 border-darkSlate-500 flex items-center gap-4 rounded-lg border p-4 transition-colors"
            onClick={() => handleSelectPlatform("modrinth")}
          >
            <img src={ModrinthLogo} class="h-8 w-8" alt="Modrinth" />
            <div class="flex flex-col items-start">
              <span class="font-medium">{t("enums:_trn_modrinth")}</span>
              <span class="text-lightSlate-500 text-sm">
                {t("instances:_trn_view_versions_on_modrinth")}
              </span>
            </div>
          </button>
        </div>

        <DialogFooter class="mt-4">
          <CancelLink onClick={() => modalContext?.closeModal()} />
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default PlatformSelection
