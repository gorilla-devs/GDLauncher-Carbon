import { keys } from "@/managers/ModalsManager/modals/OnBoarding/ThirdStep";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import { useTransContext } from "@gd/i18n";

export interface EntityCardProps {
  entity: ImportEntityStatus;
  icon: string;
  onClick?: [(_entity: ImportEntityStatus) => void, ImportEntityStatus];
  index: number;
  className?: string;
  selected?: boolean;
}

const EntityCard = (props: EntityCardProps) => {
  const [t] = useTransContext();
  return (
    <li
      class={`rounded-lg p-4 text-center ${
        props.entity.supported ? "cursor-pointer" : ""
      } gap-2 shadow-md  transform flex-col hover:bg-[#1d2029ca]  hover:shadow-lg list-none flex items-center  ${
        props.entity.supported ? "" : "bg-opacity-50"
      } backdrop-blur-lg justify-center inline-block ${
        props.className ? props.className : "h-20 w-auto"
      } bg-[#1D2028] ${
        props.selected ? "border-solid border-1 border-primary-500" : ""
      }`}
      onClick={props.onClick}
    >
      {/* <div class={`${props.icon} text-red-400 text-5xl`}></div> */}
      <img
        src={props.icon}
        alt="icon"
        class={`w-10 h-10 ${props.entity.supported ? "" : "opacity-20"}`}
      />
      <span class={`${props.entity.supported ? "" : "opacity-20"}`}>
        {t(keys[props.index])}
      </span>
    </li>
  );
};
export default EntityCard;
