import { useLocation, useNavigate } from "@solidjs/router";
import {
  Component,
  createEffect,
  createMemo,
  createSignal,
  JSX,
  Match,
  Show,
  Switch,
} from "solid-js";
import { createStore } from "solid-js/store";
import Privacypolicy from "./modals/Privacypolicy";

/**
 * It renders a modal when the URL contains a query parameter called `m`
 * @returns A component that renders a modal.
 */

interface Hash {
  [name: string]: JSX.Element;
}

const Modals: Component = () => {
  const location = useLocation();

  const navigate = useNavigate();
  const [isVisible, setIsVisible] = createSignal(false);
  const [opacity, setOpacity] = createSignal<0 | 1>(0);
  const [modals] = createStore<Hash>({
    privacyPolicy: Privacypolicy,
  });

  const queryParams = createMemo(() => location.search);
  const mParam = createMemo(() => new URLSearchParams(queryParams()).get("m"));
  const isModal = createMemo(() => mParam() !== null);

  const getModal = (type: string) => {
    const Component = () => modals[type];
    return <Component />;
  };

  createEffect(() => {
    console.log("queryParams", mParam(), getModal(mParam() || ""));
    const visibility = isModal();
    // When the URL changes, update the visibility of the modal after a timeout
    if (visibility) {
      setIsVisible(visibility);
      setTimeout(() => {
        setOpacity(1);
      }, 20);
    } else {
      setOpacity(0);
      setTimeout(() => {
        setIsVisible(visibility);
      }, 150);
    }
  });

  return (
    <div
      class="absolute opacity-0 scale-0 will-change-auto transition-opacity w-screen h-screen backdrop-blur-sm backdrop-brightness-50 grid place-items-center"
      classList={{
        "opacity-100": !!opacity(),
        "scale-100": !!opacity(),
      }}
      onClick={() => {
        navigate(location.pathname);
      }}
    >
      <Show when={isVisible()}>
        <div
          onClick={(e) => {
            e.stopPropagation();
          }}
        >
          {getModal(mParam() || "")}
        </div>
      </Show>
    </div>
  );
};

export default Modals;
