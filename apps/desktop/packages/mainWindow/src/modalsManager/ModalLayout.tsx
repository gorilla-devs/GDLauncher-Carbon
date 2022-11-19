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
    <div class={`h-130 w-190 bg-black-semiblack rounded-t-2xl ${props.class ?? ""}`}>
      <div class="h-12 w-full px-5 box-border bg-black-black rounded-t-2xl flex justify-between items-center">
        <h3>{props.title}</h3>
        <span
          class="i-gdl:close cursor-pointer"
          onClick={() => props.onClose()}
        />
      </div>
      <div class="p-5">{c()}</div>
    </div>
  );
};

export default ModalLayout;
