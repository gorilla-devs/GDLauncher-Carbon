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
      class={`flex flex-col h-fit w-fit bg-shade-7 rounded-t-2xl ${
        props.class ?? ""
      }`}
    >
      <div class="bg-shade-8 flex justify-between items-center h-12 px-5 box-border rounded-t-2xl">
        <h3>{props.title}</h3>
        <span
          class="i-gdl:close cursor-pointer"
          onClick={() => props.onClose()}
        />
      </div>
      <div class="h-full box-border p-5">{c()}</div>
    </div>
  );
};

export default ModalLayout;
