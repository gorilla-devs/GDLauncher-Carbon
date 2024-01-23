import EntityCard, { EntityCardProps } from "@/components/Card/EntityCard";
import { setPayload, payload } from "..";
import { ExportTarget } from "@gd/core_module/bindings";

export const Card = (
  props: EntityCardProps & {
    instance: {
      title: ExportTarget;
      id: number;
    };
  }
) => {
  return (
    <div
      class="flex-1"
      onClick={() => {
        setPayload({
          ...payload,
          target: props.instance.title
        });
      }}
    >
      <EntityCard
        {...props}
        selected={props.instance.title === payload.target}
      />
    </div>
  );
};
