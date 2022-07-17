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
    <Show when={isVisible()}>
      <div
        class="absolute opacity-0 transition-opacity w-screen h-screen backdrop-blur-sm backdrop-brightness-50 grid place-items-center"
        // style="transition-duration: 550ms;"
        classList={{
          "opacity-100": !!opacity(),
        }}
        onClick={() => {
          navigate(location.pathname);
        }}
      >
        <div
          class="h-40 w-40 bg-slate-100 rounded-lg"
          onClick={(e) => {
            e.stopPropagation();
          }}
        ></div>
      </div>
    </Show>
  );
};

export default Modals;
