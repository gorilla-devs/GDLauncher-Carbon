import EntityCard, { EntityCardProps } from "@/components/Card/EntityCard";
import { setPayload, payload } from "..";
import { ExportTarget } from "@gd/core_module/bindings";

export const Card = (props: EntityCardProps & { entityName: ExportTarget }) => {
  return (
    <div
      class={`h-20 w-50 flex-1 rounded-md`}
      onClick={() => {
        setPayload({ ...payload, target: props.entityName });
      }}
    >
      <EntityCard {...props} selected={props.entityName === payload.target} />
    </div>
  );
};
