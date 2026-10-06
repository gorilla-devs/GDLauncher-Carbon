import EntityCard, { EntityCardProps } from "@/components/Card/EntityCard"
import { setPayload, payload } from ".."
import { normalizeArchivePath } from "./archivePath"
import { ExportTarget } from "@gd/core_module/bindings"

export const Card = (
  props: EntityCardProps & {
    instanceTitle: ExportTarget
  }
) => {
  return (
    <div
      class="flex-1"
      onClick={() => {
        setPayload({
          ...payload,
          target: props.instanceTitle,
          save_path:
            payload.save_path === undefined
              ? undefined
              : normalizeArchivePath(payload.save_path, props.instanceTitle)
        })
      }}
    >
      <EntityCard
        {...props}
        selected={props.instanceTitle === payload.target}
      />
    </div>
  )
}
