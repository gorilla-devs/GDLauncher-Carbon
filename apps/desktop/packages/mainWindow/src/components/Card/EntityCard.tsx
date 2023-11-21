import { keys } from "@/managers/ModalsManager/modals/OnBoarding/ThirdStep";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import { useTransContext } from "@gd/i18n";

const EntityCard = (props: {
  entity: ImportEntityStatus;
  icon: string;
  onClick?: [(_entity: ImportEntityStatus) => void, ImportEntityStatus];
  index: number;
  className?: string;
}) => {
  const [t] = useTransContext();
  return (
    <li
      class={`rounded-lg ${
        props.entity.supported ? "cursor-pointer" : ""
      } gap-2 shadow-md  transform flex-col transition-transform hover:scale-105 hover:shadow-lg list-none flex items-center  ${
        props.entity.supported ? "" : "bg-opacity-50"
      } backdrop-blur-lg justify-center inline-block ${
        props.className ? props.className : "h-20 w-auto"
      } bg-[#1D2028]`}
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
