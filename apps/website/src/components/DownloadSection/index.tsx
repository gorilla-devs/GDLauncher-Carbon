import { useTranslations } from "@/i18n/utils";
import composeCDNAssetLink from "@/utils/composeCDNAssetLink";
import { Component, createSignal, Show } from "solid-js";
import DownloadedSection from "../DownloadedSection";
import Dropdown from "../Dropdown";

const DownloadSection: Component<{ pathname: string }> = ({ pathname }) => {
  const [isDownloading, setIsDownloading] = createSignal(false);

  const t = useTranslations(pathname);
  return (
    <>
      <Show
        when={!isDownloading()}
        fallback={
          <DownloadedSection
            pathname={pathname}
            setIsDownloading={() => {
              setIsDownloading(!isDownloading());
            }}
          />
        }
      >
        <div
          class={`relative flex flex-col justify-center items-center pt-1 px-10 py-10 box-border z-10 bg-[url("https://https://edge.gdlauncher.com/assets/mc_forest_2.webp")] bg-no-repeat`}
        >
          <div class="absolute top-0 bottom-0 left-0 right-0">
            <div class="absolute top-0 bottom-0 left-0 right-0 z-10 opacity-95 bg-slate-800"></div>
          </div>
          <div class="w-4/5 lg:w-full z-10 flex flex-col-reverse lg:flex-row justify-between items-center gap-10 lg:gap-12 max-w-7xl">
            <div class="py-0 px-4 max-w-2xl">
              <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                {t("download.title")}
              </h3>
              <p class="text-xl mb-6 font-light z-20">{t("download.text")}</p>
              <Dropdown
                pathname={pathname}
                onclick={() => setIsDownloading(true)}
              />
            </div>
            <div>
              <img
                class="w-full"
                src={composeCDNAssetLink("gdlauncher_download.svg")}
                alt="gdlauncher_download"
              />
            </div>
          </div>
        </div>
      </Show>
    </>
  );
};

export default DownloadSection;
