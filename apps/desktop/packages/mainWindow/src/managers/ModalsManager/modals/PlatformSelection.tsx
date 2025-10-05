import { Button } from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { useTransContext } from "@gd/i18n"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { FEUnifiedPlatform } from "@gd/core_module/bindings"

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
      title={t("instance.select_platform")}
      noPadding={false}
    >
      <div class="w-96 flex flex-col gap-4">
        <p class="text-lightSlate-400">
          {t("instance.select_platform_description", {
            modName: data().modName
          })}
        </p>

        <div class="flex flex-col gap-3 mt-4">
          <button
            class="bg-darkSlate-700 hover:bg-darkSlate-600 border-darkSlate-500 flex items-center gap-4 rounded-lg border p-4 transition-colors"
            onClick={() => handleSelectPlatform("curseforge")}
          >
            <img src={CurseforgeLogo} class="h-8 w-8" alt="CurseForge" />
            <div class="flex flex-col items-start">
              <span class="font-medium">{t("platforms.curseforge")}</span>
              <span class="text-lightSlate-500 text-sm">
                {t("instance.view_versions_on_curseforge")}
              </span>
            </div>
          </button>

          <button
            class="bg-darkSlate-700 hover:bg-darkSlate-600 border-darkSlate-500 flex items-center gap-4 rounded-lg border p-4 transition-colors"
            onClick={() => handleSelectPlatform("modrinth")}
          >
            <img src={ModrinthLogo} class="h-8 w-8" alt="Modrinth" />
            <div class="flex flex-col items-start">
              <span class="font-medium">{t("platforms.modrinth")}</span>
              <span class="text-lightSlate-500 text-sm">
                {t("instance.view_versions_on_modrinth")}
              </span>
            </div>
          </button>
        </div>

        <div class="mt-4 flex justify-end">
          <Button type="secondary" onClick={() => modalContext?.closeModal()}>
            {t("instance.cancel")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default PlatformSelection
