import { For } from "solid-js";
import { Card } from "./Card";
import {
  ExportTarget,
  ImportEntity,
  ImportEntitySelectionType,
  ImportEntityStatus
} from "@gd/core_module/bindings";
import { useTransContext } from "@gd/i18n";
import { ENTITIES } from "@/utils/constants";

const ExportFormat = () => {
  const [t] = useTransContext();

  const options: ImportEntityStatus[] = [
    {
      entity: "CurseForge",
      supported: true,
      selection_type: "file" satisfies ImportEntitySelectionType
    },
    {
      entity: "Modrinth",
      supported: true,
      selection_type: "file" satisfies ImportEntitySelectionType
    }
  ];

  const exportTargets: Partial<Record<ImportEntity, ExportTarget>> = {
    CurseForge: "Curseforge",
    Modrinth: "Modrinth"
  };

  return (
    <div class="flex flex-col">
      <span>{t("instance.export_format")}</span>
      <ul class="flex gap-2 p-0">
        <For each={options}>
          {(entity) => (
            <Card
              entity={entity}
              icon={ENTITIES[entity.entity].icon}
              translation={ENTITIES[entity.entity].translation}
              onClick={[() => {}, entity]}
              instanceTitle={exportTargets[entity.entity] as ExportTarget}
            />
          )}
        </For>
      </ul>
    </div>
  );
};

export default ExportFormat;
