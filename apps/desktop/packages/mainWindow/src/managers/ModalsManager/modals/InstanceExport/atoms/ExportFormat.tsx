import { rspc } from "@/utils/rspcClient";
import CurseForgeLogo from "/assets/images/icons/curseforge_logo.svg";
import ATLauncherLogo from "/assets/images/icons/atlauncher_logo.svg";
import FTBLogo from "/assets/images/icons/ftb_logo.svg";
import MultiMCLogo from "/assets/images/icons/multimc_logo.png";
import TechnicLogo from "/assets/images/icons/technic_logo.svg";
import PrismLogo from "/assets/images/icons/prism_logo.svg";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import LegacyGDL from "/assets/images/icons/legacy_gdlauncher.svg";
import { For } from "solid-js";
import { Card } from "./Card";
import { ExportTarget } from "@gd/core_module/bindings";
import { useTransContext } from "@gd/i18n";
const ExportFormat = () => {
  const [t] = useTransContext();
  const entities = rspc.createQuery(() => ({
    queryKey: ["instance.getImportableEntities"]
  }));

  const icons = [
    CurseForgeLogo,
    ModrinthLogo,
    LegacyGDL,
    CurseForgeLogo,
    ModrinthLogo,

    ATLauncherLogo,
    TechnicLogo,
    FTBLogo,
    MultiMCLogo,
    PrismLogo
  ];
  const instances = [
    { title: "Curseforge", id: 1 },
    { title: "Modrinth", id: 2 }
  ];
  return (
    <div class="flex flex-col">
      <span>{t("instance.export_format")}</span>
      <ul class="flex gap-2 p-0">
        <For
          each={entities.data
            ?.sort(
              (a, b) =>
                (b.supported === false ? 1 : 0) -
                (a.supported === false ? 1 : 0)
            )
            .slice(0, 2)
            .map((entity) => ({
              entity: entity.entity,
              supported: true,
              selection_type: entity.selection_type
            }))}
        >
          {(entity, i) => (
            <Card
              entity={entity}
              icon={icons[i()]}
              onClick={[() => {}, entity]}
              index={i() + 3}
              instance={instances[i()] as { title: ExportTarget; id: number }}
            />
          )}
        </For>
      </ul>
    </div>
  );
};
export default ExportFormat;
