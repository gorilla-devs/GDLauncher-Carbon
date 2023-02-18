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

const defaultModals = {
  privacyPolicy: {
    component: lazy(() => import("./modals/Privacypolicy")),
    title: "Privacy Policy",
  },
  termsAndConditions: {
    component: lazy(() => import("./modals/TermsAndConditions")),
    title: "Terms and Conditions",
  },
  addJava: {
    component: lazy(() => import("./modals/Java/AddJava")),
    title: "Add java version",
  },
  javaSetup: {
    component: lazy(() => import("./modals/Java/JavaSetup")),
    title: "Java Setup",
  },
  acceptableUsePolicy: {
    component: lazy(() => import("./modals/AcceptableUsePolicy")),
    title: "Acceptable Use Policy",
  },
};

export type ModalProps = {
  title: string;
  noHeader?: boolean;
  isVisible?: boolean;
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

type Modalskeys = keyof typeof defaultModals;

type OpenModalPath = { url: string };
type OpenModalName = { name: Modalskeys };

type Context = {
  openModal: (_modal: OpenModalPath | OpenModalName) => void;
  closeModal: () => void;
};

const ModalsContext = createContext<Context>();

export const ModalProvider = (props: { children: JSX.Element }) => {
  const [isVisible, setIsVisible] = createSignal(false);
  const navigate = useNavigate();
  const [isRoute, setIsRoute] = createSignal(false);

  const location = useLocation();
  const queryParams = () => location.search as Modalskeys;
  const mParam = () => new URLSearchParams(queryParams()).get("m");
  const [modalType, setModalType] = createSignal<Modalskeys>(
    mParam() as Modalskeys
  );
  const isModal = () => mParam() !== null;

  const noHeader = () => modals[modalType()]?.noHeader || false;
  const ModalComponent: any = () => modals[modalType()]?.component;

  const title = () => modals[modalType()]?.title;

  const [modals] = createStore<Hash>(defaultModals);

  function isPath(data: OpenModalPath | OpenModalName): data is OpenModalPath {
    if ((data as OpenModalPath).url) {
      return true;
    }
    return false;
  }

  const manager = {
    openModal: (modal: OpenModalPath | OpenModalName) => {
      const modalParamRegex = /m=([^&]+)/;

      if (isPath(modal)) {
        const mParam = () =>
          modal.url.match(modalParamRegex)?.[1] as Modalskeys;
        const isModal = () => mParam() !== null;

        if (isModal()) {
          setIsRoute(true);
          navigate(modal.url);
        }
      } else {
        setIsRoute(false);
        setModalType(modal.name);
        setIsVisible(true);
      }
    },
    closeModal: () => {
      setIsRoute(false);
      setIsVisible(false);
    },
  };

  createEffect(() => {
    if (mParam() && mParam() !== modalType() && isRoute()) {
      setModalType(mParam() as Modalskeys);
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
