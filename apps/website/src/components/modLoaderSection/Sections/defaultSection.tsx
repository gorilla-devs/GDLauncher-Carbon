import { useTranslations } from "@/i18n/utils";
import composeCDNAssetLink from "@/utils/composeCDNAssetLink";

function DefaultSection() {
    const t = useTranslations(window.location.pathname);
  
    return (
      <div class="pt-32">
        <div class="w-4/5 lg:w-full z-20 flex flex-col-reverse lg:flex-row justify-between items-center gap-20 max-w-7xl">
          <div class="py-0 px-4 max-w-2xl">
            <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
              {t("modloader.vanilla_title")}
            </h3>
            <p class="text-xl mb-12 font-light">{t("modloader.vanilla_text")}</p>
          </div>
          <div>
            <img
              class="w-full"
              src={composeCDNAssetLink("all_platforms_covered.webp")}
              alt="launcher_mockup"
            />
          </div>
        </div>
      </div>
    );
  }

  export default DefaultSection