import { children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element | Element;
  class?: string;
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children);
  return (
    <div class={`h-130 w-190 bg-semiblack rounded-lg ${props.class ?? ""}`}>
      {c()}
    </div>
  );
};

export default ModalLayout;
