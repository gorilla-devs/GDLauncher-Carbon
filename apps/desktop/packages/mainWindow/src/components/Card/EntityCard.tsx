import { ImportEntityStatus } from "@gd/core_module/bindings";

const EntityCard = (props: {
  entity: ImportEntityStatus;
  icon: string;
  onClick?: [(entity: ImportEntityStatus) => void, ImportEntityStatus];
}) => {
  return (
    <li
      class={`rounded-lg ${
        props.entity.supported ? "cursor-pointer" : ""
      } gap-2 shadow-md border-neutral-800 border-2 border-solid transform flex-col transition-transform hover:scale-105 hover:shadow-lg list-none flex items-center bg-opacity-60 justify-center  w-auto inline-block h-20 bg-gray-900`}
      onClick={props.onClick}
    >
      {/* <div class={`${props.icon} text-red-400 text-5xl`}></div> */}
      <img src={props.icon} alt="icon" class="w-10 h-10" />
      <span>{props.entity.entity}</span>
    </li>
  );
};
export default EntityCard;
