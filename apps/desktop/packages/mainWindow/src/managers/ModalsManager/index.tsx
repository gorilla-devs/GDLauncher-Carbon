import { useLocation, useSearchParams } from "@solidjs/router";
import {
  Accessor,
  createContext,
  createSignal,
  JSX,
  lazy,
  Show,
  Suspense,
  useContext,
} from "solid-js";
import { Dynamic, Portal } from "solid-js/web";
import { useGDNavigate } from "../NavigationManager";

export type ModalProps = {
  title: string;
  noHeader?: boolean;
  data?: any;
};

type Hash = {
  [name: string]: {
    component: ((_props: ModalProps) => JSX.Element) & {
      preload: () => Promise<{ default: (_props: ModalProps) => JSX.Element }>;
    };

    title?: string;
    noHeader?: boolean;
  };
};

const defaultModals: Hash = {
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
  addMod: {
    component: lazy(() => import("./modals/AddMod")),
    title: "Add mod",
  },
  modDetails: {
    component: lazy(() => import("./modals/ModDetails")),
    title: "Mod Details",
  },
  javaSetup: {
    component: lazy(() => import("./modals/Java/JavaSetup")),
    title: "Java Setup",
  },
  acceptableUsePolicy: {
    component: lazy(() => import("./modals/AcceptableUsePolicy")),
    title: "Acceptable Use Policy",
  },
  instanceCreation: {
    component: lazy(() => import("./modals/InstanceCreation")),
    title: "New Instance",
  },
  logViewer: {
    component: lazy(() => import("./modals/LogViewer")),
    title: "Logs",
  },
  notification: {
    component: lazy(() => import("./modals/Notification")),
    title: "Notification",
  },
  onBoarding: {
    component: lazy(() => import("./modals/OnBoarding")),
    noHeader: true,
  },
};

type ModalName = string;

type Modal = { name: ModalName; url?: string };

type Context = {
  openModal: (_modal: Modal, _data?: any) => void;
  closeModal: () => void;
  isVisible: Accessor<boolean>;
};

const ModalsContext = createContext<Context>();

export const ModalProvider = (props: { children: JSX.Element }) => {
  const navigate = useGDNavigate();
  const [isVisible, setIsVisible] = createSignal(false);
  const location = useLocation();
  const queryParams = () => location.search as ModalName;
  const urlSearchParams = () => new URLSearchParams(queryParams());
  const mParam = () => urlSearchParams().get("m");
  const [data, setData] = createSignal<any>(undefined);

  const [_searchParams, setSearchParams] = useSearchParams();

  const modalTypeIndex = () => mParam() || "";
  const noHeader = () => defaultModals[modalTypeIndex()]?.noHeader || false;

  const ModalComponent: any = () => defaultModals[modalTypeIndex()]?.component;

  const title = () => defaultModals[modalTypeIndex()]?.title;

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay") as HTMLElement;
      overlay.style.display = "flex";
      setData(data);
      if (modal.url) {
        const url = new URLSearchParams(modal.url);
        url.append("m", modal.name);

        const decodedParamString = decodeURIComponent(url.toString());

        navigate(decodedParamString.replace("=&", "?"));
        setTimeout(() => {
          setIsVisible(true);
        }, 100);
      } else {
        setSearchParams({ m: modal.name });
        setTimeout(() => {
          setIsVisible(true);
        }, 100);
      }
    },
    closeModal: () => {
      setIsVisible(false);
      setTimeout(() => {
        urlSearchParams()?.delete("m");
        const overlay = document.getElementById("overlay") as HTMLElement;
        overlay.style.display = "none";
      }, 100);
    },
    isVisible,
  };

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <Show when={mParam()}>
          {/* <Suspense fallback={<p>Loading...</p>}> */}
          <div class="h-screen w-screen">
            <Dynamic
              component={ModalComponent({
                noHeader,
                title,
              })}
              data={data()}
              noHeader={noHeader()}
              title={title()}
            />
          </div>
          {/* </Suspense> */}
        </Show>
      </Portal>
    </ModalsContext.Provider>
  );
};

export const useModal = () => {
  return useContext(ModalsContext);
};
