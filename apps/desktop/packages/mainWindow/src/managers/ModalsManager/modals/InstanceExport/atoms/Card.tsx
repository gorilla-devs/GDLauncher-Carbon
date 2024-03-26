import EntityCard, { EntityCardProps } from "@/components/Card/EntityCard";
import { setPayload, payload } from "..";
import { ExportTarget } from "@gd/core_module/bindings";

export const Card = (
  props: EntityCardProps & {
    instanceTitle: ExportTarget;
  }
) => {
  return (
    <div
      class="flex-1"
      onClick={() => {
        setPayload({
          ...payload,
          target: props.instanceTitle
        });
      }}
    >
      <EntityCard
        {...props}
        selected={props.instanceTitle === payload.target}
      />
    </div>
  );
};
