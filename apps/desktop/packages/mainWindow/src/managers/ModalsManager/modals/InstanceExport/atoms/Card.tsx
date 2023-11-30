import EntityCard, { EntityCardProps } from "@/components/Card/EntityCard";
import { setPayload, payload } from "..";
import { ExportTarget } from "@gd/core_module/bindings";
import { createEffect } from "solid-js";
import { rspc } from "@/utils/rspcClient";

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
      class={`h-20 w-50 flex-1 rounded-md`}
      onClick={() => {
        setPayload({
          ...payload,
          target: props.instance.title,
          instance_id: props.instance.id
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
