import { useLocation } from "@solidjs/router";
import { Show, children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";
import { ModalProps, useModal } from ".";
import { useGdNavigation } from "../NavigationManager";

interface Props extends ModalProps {
  children: JSX.Element | Element;
  class?: string;
  preventClose?: boolean;
  noPadding?: boolean;
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children);
  const navigate = useGdNavigation();
  const location = useLocation();
  const modalsContext = useModal();

  return (
    <div class="w-screen h-screen">
      <div
        class="h-screen w-screen absolute text-white ease-in-out duration-100 will-change-auto transition-opacity backdrop-blur-sm backdrop-brightness-50 grid place-items-center z-999 transition-opacity origin-center"
        classList={{
          "opacity-100": modalsContext?.isVisible(),
          "opacity-0": !modalsContext?.isVisible(),
        }}
        onClick={() => {
          if (!props.preventClose) {
            navigate?.navigate(location.pathname);
            modalsContext?.closeModal();
          }
        }}
      >
        <div
          onClick={(e) => {
            e.stopPropagation();
          }}
        >
          <div
            class={`flex flex-col h-fit w-fit bg-shade-7 rounded-2xl transition-scale ease-in-out origin-center duration-100 overflow-hidden ${
              props.class ?? ""
            }`}
            classList={{
              "scale-100": modalsContext?.isVisible(),
              "scale-0": !modalsContext?.isVisible(),
            }}
          >
            <Show when={!props.noHeader}>
              <div class="bg-shade-8 flex justify-between items-center px-5 box-border h-12 rounded-t-2xl">
                <h3>{props.title}</h3>
                <div
                  class="cursor-pointer text-shade-5 h-5 w-5 i-ri:close-fill"
                  onClick={() => {
                    if (!props.preventClose) {
                      navigate?.navigate(location.pathname);
                      modalsContext?.closeModal();
                    }
                  }}
                />
              </div>
            </Show>
            <div
              class="box-border h-full overflow-hidden"
              classList={{
                "p-5": !props.noPadding,
              }}
            >
              {c()}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ModalLayout;
