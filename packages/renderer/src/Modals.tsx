import { useLocation, useNavigate } from "solid-app-router";
import { Component, createEffect, Match, Show, Switch } from "solid-js";

const Modals: Component = () => {
  const location = useLocation();
  const isModal = () => new URLSearchParams(location.search).get("m") !== null;

  const navigate = useNavigate();

  return (
    <Show when={isModal()}>
      <div
        class="absolute opacity-0 transition-opacity w-screen h-screen backdrop-blur-sm backdrop-brightness-50"
        classList={{
          "opacity-100": isModal(),
        }}
      >
        Ciao
        <button
          onClick={() => {
            navigate(location.pathname);
          }}
        >
          Close
        </button>
      </div>
    </Show>
  );
};

export default Modals;
