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

type OpenModal = { name: Modalskeys; url?: string };

type Context = {
  openModal: (_modal: OpenModal) => void;
  closeModal: () => void;
};

const ModalsContext = createContext<Context>();

export const ModalProvider = (props: { children: JSX.Element }) => {
  const navigate = useNavigate();

  const location = useLocation();
  const queryParams = () => location.search as Modalskeys;
  const mParam = () => new URLSearchParams(queryParams()).get("m");
  const [modalType, setModalType] = createSignal<Modalskeys | null>(
    mParam() as Modalskeys
  );

  const modalTypeIndex = () => modalType() || "";
  const noHeader = () => modals[modalTypeIndex()]?.noHeader || false;
  const ModalComponent: any = () => modals[modalTypeIndex()]?.component;

  const title = () => modals[modalTypeIndex()]?.title;

  const [modals] = createStore<Hash>(defaultModals);

  const manager = {
    openModal: (modal: OpenModal) => {
      const modalParamRegex = /m=([^&]+)/;

      if (modal.url) {
        const mParam = modal.url.match(modalParamRegex)?.[1] as Modalskeys;
        const isModal = mParam !== null;

        if (isModal) {
          navigate(modal.url);
        }
      }

      setModalType(modal.name);
    },
    closeModal: () => {
      setModalType(null);
    },
  };

  createEffect(() => {
    if (mParam() && mParam() !== modalType()) {
      setModalType(mParam() as Modalskeys);
    }
  });

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <Show when={modalType()}>
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
