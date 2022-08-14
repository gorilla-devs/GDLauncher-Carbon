import { useTranslations } from "@/i18n/utils";
import composeCDNAssetLink from "@/utils/composeCDNAssetLink";
import { t } from "i18next";
import { mergeProps, createSignal, Switch, Match } from "solid-js";
import DefaultSection from "./Sections/defaultSection";

enum sectionType {
  vanilla = "vanilla",
  forge = "forge",
  fabric = "fabric",
  none = "none",
}

type Props = {
  t: any;
};

function Section(props: { type: sectionType }) {
  const t = useTranslations(window.location.pathname);

  const reactiveProps = mergeProps(props);

  return (
    <Switch fallback={<DefaultSection />}>
      <Match when={reactiveProps.type === sectionType.vanilla}>
        <div class="pt-32">
          <div class="w-4/5 lg:w-full z-20 flex flex-col-reverse lg:flex-row justify-between items-center gap-20 max-w-7xl">
            <div class="py-0 px-4 max-w-2xl">
              <h3 class="text-5xl lg:text-8xl mb-2 font-bold">
                {t("modloader.vanilla_title")}
              </h3>
              <p class="text-xl mb-12 font-light">
                {t("modloader.vanilla_text")}
              </p>
            </div>
            <div>
              <img
                class="w-full"
                src={composeCDNAssetLink("launcher_mockup.webp")}
                alt="launcher_mockup"
              />
            </div>
          </div>
        </div>
      </Match>
      <Match when={reactiveProps.type === sectionType.forge}>
        <div>Forge</div>
      </Match>
      <Match when={reactiveProps.type === sectionType.fabric}>
        <div>Fabric</div>
      </Match>
    </Switch>
  );
}

function ModLoaderSection() {
  const [type, setType] = createSignal<sectionType>(sectionType.none);
  return (
    <div class="h-auto flex justify-center items-start py-32">
      <div class="flex flex-col items-center">
        <div class="flex flex-row gap-5 uppercase font-medium text-xl">
          <div
            class={`relative w-32 flex flex-col items-center cursor-pointer ${
              type() === sectionType.vanilla ? "font-bold" : ""
            } ease-in-out duration-100`}
            onClick={() => setType(sectionType.vanilla)}
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
            onClick={() => setType(sectionType.forge)}
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
            onClick={() => setType(sectionType.fabric)}
          >
            <h2 class="pb-3">Fabric</h2>
            <div
              class={`w-full h-2 ${
                type() === sectionType.fabric ? "bg-yellow-400" : "bg-blue-600"
              }`}
            />
          </div>
        </div>

        <Section type={type()} />
      </div>
    </div>
  );
}

export default ModLoaderSection;
