import { useLocation, useNavigate } from "@solidjs/router";
import {
  createContext,
  createEffect,
  createSignal,
  JSX,
  lazy,
  Show,
  Suspense,
  useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { Dynamic, Portal } from "solid-js/web";

export type ModalProps = {
  title: string;
  noHeader?: boolean;
  isVisible?: boolean;
  opacity?: number;
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

type Context = {
  openModal: (_modal: string) => void;
  closeModal: () => void;
};

const ModalsContext = createContext<Context>();

export const ModalProvider = (props: { children: JSX.Element }) => {
  const [isVisible, setIsVisible] = createSignal(false);
  const navigate = useNavigate();
  const [isRoute, setIsRoute] = createSignal(false);

  const location = useLocation();
  const queryParams = () => location.search;
  const mParam = () => new URLSearchParams(queryParams()).get("m");
  const [modalType, setModalType] = createSignal(mParam() || "");
  const isModal = () => mParam() !== null;

  const noHeader = () => modals[modalType()]?.noHeader || false;
  const ModalComponent: any = () => modals[modalType()]?.component;

  const defaultModals = {
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
  };

  const title = () => modals[modalType()]?.title;

  const [modals] = createStore<Hash>(defaultModals);

  type modalskeys = keyof typeof defaultModals;

  const manager = {
    openModal: (modal: modalskeys | string) => {
      const urlPathRegex =
        /^\/[a-zA-Z0-9-_]+(?:\/[a-zA-Z0-9-_]+)*(?:\?[a-zA-Z0-9-_]+=[a-zA-Z0-9-_]+(?:&[a-zA-Z0-9-_]+=[a-zA-Z0-9-_]+)*)?$/;

      const modalParamRegex = /m=([^&]+)/;

      const isPath = () => urlPathRegex.test(modal);
      const mParam = () => modal.match(modalParamRegex)?.[1];
      const isModal = () => mParam() !== null;

      if (isPath() && isModal()) {
        setIsRoute(true);
        navigate(modal);
      } else {
        setIsRoute(false);
        setModalType(modal);
        setIsVisible(true);
      }
    },
    closeModal: () => {
      setIsRoute(false);
      setModalType("");
      setIsVisible(false);
    },
  };

  createEffect(() => {
    if (mParam() && mParam() !== modalType() && isRoute()) {
      setModalType(mParam() || "");
    }
  });

  createEffect(() => {
    const visibility = isModal();
    // When the URL changes, update the visibility of the modal after a timeout
    if (visibility) {
      setIsVisible(visibility);
    } else {
      setTimeout(() => {
        setIsVisible(visibility);
      }, 150);
    }
  });

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <Show when={isVisible()}>
          <Suspense fallback={<p>Loading...</p>}>
            <Dynamic
              component={ModalComponent({ noHeader, title })}
              noHeader={noHeader()}
              title={title()}
            />
          </Suspense>
        </Show>
      </Portal>
    </ModalsContext.Provider>
  );
};

export const useModal = () => {
  return useContext(ModalsContext);
};
