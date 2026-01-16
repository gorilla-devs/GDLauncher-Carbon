import { ImportEntityStatus } from "@gd/core_module/bindings"
import { useTransContext, NamespacedTranslationKey } from "@gd/i18n"
import { PRESS_CLASSES } from "@gd/ui"
import { Show } from "solid-js"

export interface EntityCardProps {
  entity: ImportEntityStatus
  icon: string
  onClick?: [(_entity: ImportEntityStatus) => void, ImportEntityStatus]
  translation: NamespacedTranslationKey
  className?: string
  selected?: boolean
}

const EntityCard = (props: EntityCardProps) => {
  const [t] = useTransContext()
  return (
    <li
      class={`rounded-lg p-2 text-center h-20 ${
        props.entity.supported ? "cursor-pointer" : ""
      } flex-col gap-1.5 shadow-md ${
        props.entity.selection_type
          ? "outline outline-1 outline-transparent hover:outline-darkSlate-500"
          : ""
      } flex list-none items-center hover:shadow-lg ${
        props.entity.supported ? "" : "bg-opacity-50"
      } justify-center backdrop-blur-lg ${
        props.className ? props.className : ""
      } bg-darkSlate-800 ${
        props.selected ? "border-1 border-primary-500 border-solid" : ""
      } ${props.entity.supported ? PRESS_CLASSES : ""}`}
      onClick={props.onClick}
    >
      <Show when={!props.entity.supported}>
        <span class="text-xs font-bold text-teal-600">
          {t("tracking:_trn_soon")}
        </span>
      </Show>
      <div class="relative">
        <img
          src={props.icon}
          alt="icon"
          class={`h-7 w-7 ${props.entity.supported ? "" : "opacity-20"}`}
        />
      </div>

      <span class={`text-xs ${props.entity.supported ? "" : "opacity-20"}`}>
        {t(props.translation)}
      </span>
    </li>
  )
}
export default EntityCard
