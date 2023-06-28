import { useLocation } from "@solidjs/router";
import { Show, children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";
import { ModalProps, useModal } from ".";
import { useGDNavigate } from "../NavigationManager";

interface Props extends ModalProps {
  children: JSX.Element | Element;
  class?: string;
  preventClose?: boolean;
  noPadding?: boolean;
  overflowHiddenDisabled?: boolean;
  background?: JSX.Element;
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children);
  const navigate = useGDNavigate();
  const location = useLocation();
  const modalsContext = useModal();

  return (
    <div
      onClick={(e) => {
        e.stopPropagation();
      }}
    >
      <div
        class={`relative flex flex-col h-fit w-fit bg-darkSlate-700 rounded-2xl ease-in-out origin-center duration-100 ${
          props.class ?? ""
        }`}
        classList={{
          "overflow-hidden": !props.overflowHiddenDisabled,
        }}
      >
        <Show when={props.background}>{props.background}</Show>
        <Show when={!props.noHeader}>
          <div class="bg-darkSlate-800 flex justify-between items-center px-5 box-border h-12 rounded-t-2xl">
            <h3>{props.title}</h3>
            <div
              class="cursor-pointer text-darkSlate-500 h-5 w-5 i-ri:close-fill"
              onClick={() => {
                navigate(location.pathname);
                modalsContext?.closeModal();
              }}
            />
          </div>
        </Show>
        <div
          class="box-border h-full overflow-hidden z-10"
          classList={{
            "p-5": !props.noPadding,
          }}
        >
          {c()}
        </div>
      </div>
    </div>
  );
};

export default ModalLayout;
