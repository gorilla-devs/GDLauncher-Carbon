import { useTranslations } from "@/i18n/utils";
import composeCDNAssetLink from "@/utils/composeCDNAssetLink";
import { mergeProps, createSignal, Switch, Match, Component } from "solid-js";
import "./style.scss";

enum sectionType {
  vanilla = "vanilla",
  forge = "forge",
  fabric = "fabric",
  none = "none",
}

interface Props {
  t: any;
}

const images = {
  vanilla: "vanilla_integration.webp",
  forge: "forge_integration.webp",
  fabric: "fabric_integration.webp",
  none: "all_platforms_covered.webp",
};

function Section(props: { type: sectionType } & Props) {
  const reactiveProps = mergeProps(props);

  const t = reactiveProps.t;

  return (
    <div class="pt-32 flex flex-col-reverse lg:flex-row justify-center lg:justify-between items-center">
      <div class="relatove w-4/5 lg:w-full z-20 max-w-7xl flex flex-col-reverse lg:flex-row justify-center lg:justify-between items-center">
        <div class="absolute py-0 px-4 max-w-2xl z-30">
          <Switch
            fallback={
              <>
                <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                  {t("modloader.none_title")}
                </h3>
                <p class="text-xl max-w-lg mb-12 font-light">
                  {t("modloader.none_text")}
                </p>
              </>
            }
          >
            <Match when={reactiveProps.type === sectionType.vanilla}>
              <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                {t("modloader.vanilla_title")}
              </h3>
              <p class="text-xl max-w-lg mb-12 font-light">
                {t("modloader.vanilla_text")}
              </p>
            </Match>
            <Match when={reactiveProps.type === sectionType.forge}>
              <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                {t("modloader.forge_title")}
              </h3>
              <p class="text-xl max-w-lg mb-12 font-light">
                {t("modloader.forge_text")}
              </p>
            </Match>
            <Match when={reactiveProps.type === sectionType.fabric}>
              <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                {t("modloader.fabric_title")}
              </h3>
              <p class="text-xl max-w-lg mb-12 font-light">
                {t("modloader.fabric_text")}
              </p>
            </Match>
          </Switch>
        </div>
        <div class="lg:m-h-[540px]">
          <img
            class="w-full opacity-10 lg:opacity-100 h-[140px] lg:h-[540px]"
            loading="lazy"
            src={composeCDNAssetLink(images[reactiveProps.type])}
            alt="launcher_mockup"
          />
        </div>
      </div>
    </div>
  );
}

const ModLoaderSection: Component<{ pathname: string }> = ({ pathname }) => {
  const [type, setType] = createSignal<sectionType>(sectionType.none);

  const t = useTranslations(pathname);

  const handeClick = (sectionTypeProp: sectionType) => {
    if (type() === sectionTypeProp) {
      setType(sectionType.none);
    } else setType(sectionTypeProp);
  };

  return (
    <div class="h-auto flex justify-center items-start py-32">
      <div class="flex flex-col items-center">
        <div class="flex flex-row lg:gap-5 uppercase font-medium text-xl">
          <div
            class={`relative w-32 flex flex-col items-center cursor-pointer ${
              type() === sectionType.vanilla ? "font-bold" : ""
            } ease-in-out duration-100`}
            onClick={() => handeClick(sectionType.vanilla)}
          >
            <h2 class="pb-3">Vanilla</h2>
            <div
              class={`w-full h-2 ${
                type() === sectionType.vanilla ? "bg-yellow-400" : "bg-blue-600"
              }`}
            />
          </div>
          <div
            class={`relative w-32 flex flex-col items-center cursor-pointer ${
              type() === sectionType.forge ? "font-bold" : ""
            } ease-in-out duration-100`}
            onClick={() => handeClick(sectionType.forge)}
          >
            <h2 class="pb-3">Forge</h2>
            <div
              class={`w-full h-2 ${
                type() === sectionType.forge ? "bg-yellow-400" : "bg-blue-600"
              }`}
            />
          </div>
          <div
            class={`relative w-32 flex flex-col items-center cursor-pointer ${
              type() === sectionType.fabric ? "font-bold" : ""
            } ease-in-out duration-100`}
            onClick={() => handeClick(sectionType.fabric)}
          >
            <h2 class="pb-3">Fabric</h2>
            <div
              class={`w-full h-2 ${
                type() === sectionType.fabric ? "bg-yellow-400" : "bg-blue-600"
              }`}
            />
          </div>
        </div>
        <div>
          <Section type={type()} t={t || (() => {})} />
        </div>
      </div>
    </div>
  );
};

export default ModLoaderSection;
