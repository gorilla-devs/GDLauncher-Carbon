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
import EntityCard from "@/components/Card/EntityCard";
const ExportFormat = () => {
  const entities = rspc.createQuery(() => ["instance.getImportableEntities"]);
  const icons = [
    LegacyGDL,
    CurseForgeLogo,
    ModrinthLogo,
    CurseForgeLogo,
    ModrinthLogo,
    ATLauncherLogo,
    TechnicLogo,
    FTBLogo,
    MultiMCLogo,
    PrismLogo
  ];
  return (
    <div class="flex">
      <ul class="flex overflow-x-scroll overflow-y-hidden gap-2 p-0">
        <For
          each={entities.data
            ?.sort(
              (a, b) =>
                (b.supported === true ? 1 : 0) - (a.supported === true ? 1 : 0)
            )
            .slice(0, 3)}
        >
          {(entity, i) => (
            <EntityCard
              entity={entity}
              icon={icons[i()]}
              onClick={[() => {}, entity]}
              index={i()}
              className="h-20 w-40"
            />
          )}
        </For>
      </ul>
    </div>
  );
};
export default ExportFormat;
