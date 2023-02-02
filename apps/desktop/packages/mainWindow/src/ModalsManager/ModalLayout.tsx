import { children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element | Element;
  class?: string;
  title?: string;
  onClose: () => void;
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children);
  return (
    <div
      class={`flex flex-col h-130 w-190 bg-shade-7 rounded-t-2xl ${
        props.class ?? ""
      }`}
    >
      <div class="h-12 w-full px-5 box-border bg-shade-8 rounded-t-2xl flex justify-between items-center">
        <h3>{props.title}</h3>
        <span
          class="i-gdl:close cursor-pointer"
          onClick={() => props.onClose()}
        />
      </div>
      <div class="p-5 h-full box-border">{c()}</div>
    </div>
  );
};

export default ModalLayout;
