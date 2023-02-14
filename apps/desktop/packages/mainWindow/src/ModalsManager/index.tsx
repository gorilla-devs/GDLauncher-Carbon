/* eslint-disable @unocss/order */
import { useLocation } from "@solidjs/router";
import {
  Component,
  createEffect,
  createSignal,
  JSX,
  lazy,
  Show,
  Suspense,
} from "solid-js";
import { createStore } from "solid-js/store";
import { Dynamic } from "solid-js/web";

/**
 * It renders a modal when the URL contains a query parameter called `m`
 * @returns A component that renders a modal.
 */

export type ModalProps = {
  title?: string;
  noHeader?: boolean;
  isVisible: boolean;
  opacity: number;
};

type Hash = {
  [name: string]: {
    component: ((_props: ModalProps) => JSX.Element) & {
      preload: () => Promise<{ default: (_props: ModalProps) => JSX.Element }>;
    };

    title: string;
    noHeader?: boolean;
  };
};

const Modals: Component = () => {
  const location = useLocation();

  const [isVisible, setIsVisible] = createSignal(false);
  const [opacity, setOpacity] = createSignal<0 | 1>(0);
  const [modals] = createStore<Hash>({
    privacyPolicy: {
      component: lazy(() => import("./modals/Privacypolicy")),
      title: "Privacy Policy",
    },
    termsAndConditions: {
      component: lazy(() => import("./modals/TermsAndConditions")),
      title: "Terms and Conditions",
    },
    addjava: {
      component: lazy(() => import("./modals/Java/AddJava")),
      title: "Add java version",
    },
    javasetup: {
      component: lazy(() => import("./modals/Java/JavaSetup")),
      title: "Java Setup",
    },
    acceptableUsePolicy: {
      component: lazy(() => import("./modals/AcceptableUsePolicy")),
      title: "Acceptable Use Policy",
    },
  });

  const queryParams = () => location.search;
  const mParam = () => new URLSearchParams(queryParams()).get("m");
  const isModal = () => mParam() !== null;

  const type = () => mParam() || "";
  const noHeader = () => modals[type()]?.noHeader || false;
  const ModalComponent: any = () => modals[type()]?.component;
  const title = () => modals[type()]?.title;

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
      <Suspense fallback={<p>Loading...</p>}>
        <Dynamic
          component={ModalComponent({ noHeader, title })}
          noHeader={noHeader()}
          title={title()}
        />
      </Suspense>
    </Show>
  );
};

export default Modals;
