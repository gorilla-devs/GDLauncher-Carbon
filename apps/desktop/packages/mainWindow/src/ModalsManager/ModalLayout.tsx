import { useLocation, useNavigate } from "@solidjs/router";
import { Show, children } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";

interface Props {
  children: JSX.Element | Element;
  class?: string;
  title?: string;
  noHeader?: boolean;
  preventClose?: boolean;
  noPadding?: boolean;
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children);
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <div
      class="h-screen absolute opacity-100 will-change-auto transition-opacity w-screen backdrop-blur-sm backdrop-brightness-50 grid place-items-center text-white z-999 scale-100"
      onClick={() => {
        if (!props.preventClose) navigate(location.pathname);
      }}
    >
      <div
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        <div
          class={`flex flex-col h-fit w-fit bg-shade-7 rounded-2xl ${
            props.class ?? ""
          }`}
        >
          <Show when={!props.noHeader}>
            <div class="bg-shade-8 flex justify-between items-center h-12 px-5 box-border rounded-t-2xl">
              <h3>{props.title}</h3>
              <div
                class="cursor-pointer text-shade-5 h-5 w-5 i-ri:close-fill"
                onClick={() => {
                  if (!props.preventClose) navigate(location.pathname);
                }}
              />
            </div>
          </Show>
          <div
            class="box-border h-full"
            classList={{
              "p-5": !props.noPadding,
            }}
          >
            {c()}
          </div>
        </div>
      </div>
    </div>
  );
};

export default ModalLayout;
