import { ImportEntityStatus } from "@gd/core_module/bindings"
import { useTransContext, NamespacedTranslationKey } from "@gd/i18n"
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
      class={`h-32 w-32 rounded-lg p-4 text-center ${
        props.entity.supported ? "cursor-pointer" : ""
      } flex-col gap-3  shadow-md ${
        props.entity.selection_type ? "hover:bg-[#1d2029ca]" : ""
      }  flex list-none items-center hover:shadow-lg  ${
        props.entity.supported ? "" : "bg-opacity-50"
      } inline-block justify-center backdrop-blur-lg ${
        props.className ? props.className : "h-20 w-auto"
      } bg-[#1D2028] ${
        props.selected ? "border-1 border-primary-500 border-solid" : ""
      }`}
      onClick={props.onClick}
    >
      {/* <div class={`${props.icon} text-red-400 text-5xl`}></div> */}
      {/* absolute left-0 right-0 text-center ml-auto mr-auto top-[30%] */}
      <Show when={!props.entity.supported}>
        <span class="font-bold text-teal-600">{t("tracking:_trn_soon")}</span>
      </Show>
      <div class="relative">
        <img
          src={props.icon}
          alt="icon"
          class={`h-10 w-10 ${props.entity.supported ? "" : "opacity-20"}`}
        />
      </div>

      <span class={`${props.entity.supported ? "" : "opacity-20"}`}>
        {t(props.translation)}
      </span>
    </li>
  )
}
export default EntityCard
