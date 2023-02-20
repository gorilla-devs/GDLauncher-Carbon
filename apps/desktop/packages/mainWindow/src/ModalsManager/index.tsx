import { useLocation, useNavigate, useSearchParams } from "@solidjs/router";
import {
  createContext,
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
  const urlSearchParans = () => new URLSearchParams(queryParams());
  const mParam = () => urlSearchParans().get("m");

  const [_searchParams, setSearchParams] = useSearchParams();

  const modalTypeIndex = () => mParam() || "";
  const noHeader = () => modals[modalTypeIndex()]?.noHeader || false;
  const ModalComponent: any = () => modals[modalTypeIndex()]?.component;

  const title = () => modals[modalTypeIndex()]?.title;

  const [modals] = createStore<Hash>(defaultModals);

  const manager = {
    openModal: (modal: OpenModal) => {
      if (modal.url) {
        const url = new URLSearchParams(modal.url);
        url.append("m", modal.name);

        const decodedParamString = decodeURIComponent(url.toString());

        navigate(decodedParamString.replace("=&", "?"));
      } else {
        setSearchParams({ m: modal.name });
      }
    },
    closeModal: () => {
      urlSearchParans()?.delete("m");
    },
  };

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <Show when={mParam()}>
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
