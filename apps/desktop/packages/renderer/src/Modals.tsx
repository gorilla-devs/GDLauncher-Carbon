import { useLocation, useNavigate } from "solid-app-router";
import {
  Component,
  createEffect,
  createMemo,
  createSignal,
  Match,
  Show,
  Switch,
} from "solid-js";

/**
 * It renders a modal when the URL contains a query parameter called `m`
 * @returns A component that renders a modal.
 */

const Modals: Component = () => {
  const location = useLocation();

  const navigate = useNavigate();
  const [isVisible, setIsVisible] = createSignal(false);
  const [opacity, setOpacity] = createSignal<0 | 1>(0);

  const queryParams = createMemo(() => location.search);
  const isModal = createMemo(
    () => new URLSearchParams(location.search).get("m") !== null
  );

  createEffect(() => {
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
          class="h-40 w-40 bg-slate-100 rounded-lg"
          onClick={(e) => {
            e.stopPropagation();
          }}
        ></div>
      </Show>
    </div>
  );
};

export default Modals;
