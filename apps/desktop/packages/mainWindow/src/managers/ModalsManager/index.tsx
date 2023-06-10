import { useLocation, useSearchParams } from "@solidjs/router";
import {
  createContext,
  createEffect,
  createSignal,
  For,
  JSX,
  lazy,
  Show,
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

type ModalName = {
  [K in keyof typeof defaultModals as string extends K ? K : never]: K;
}[keyof typeof defaultModals];

type Modal = { name: ModalName; url?: string };

type Context = {
  openModal: (_modal: Modal, _data?: any) => void;
  closeModal: () => void;
};

type Stack = Array<{ name: ModalName; data: any }>;

const ModalsContext = createContext<Context>();

export const ModalProvider = (props: { children: JSX.Element }) => {
  const navigate = useGDNavigate();
  const location = useLocation();
  const queryParams = () => location.search as ModalName;
  const urlSearchParams = () => new URLSearchParams(queryParams());
  const [modalStack, setModalStack] = createSignal<Stack>([]);

  const [_searchParams, setSearchParams] = useSearchParams();

  const closeModal = (name?: ModalName) => {
    setModalStack((currentStack) => {
      if (name) {
        // Remove the specified modal
        return currentStack.filter((modal) => modal.name !== name);
      } else {
        // Remove the top modal
        return currentStack.slice(0, currentStack.length - 1);
      }
    });

    if (modalStack().length === 0) {
      urlSearchParams()?.delete("m");
      const overlay = document.getElementById("overlay") as HTMLElement;
      overlay.style.display = "none";
    }
  };

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay") as HTMLElement;
      overlay.style.display = "flex";
      setModalStack((currentStack) => [
        ...currentStack,
        { name: modal.name, data },
      ]);

      if (modal.url) {
        const url = new URLSearchParams(modal.url);
        url.append("m", modal.name);

        const decodedParamString = decodeURIComponent(url.toString());
        navigate(decodedParamString.replace("=&", "?"));
      } else {
        setSearchParams({ m: modal.name });
      }
    },
    closeModal,
  };

  createEffect(() => {
    console.log("STACK", modalStack());
  });

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <div class="h-screen w-screen">
          <div
            class="h-screen w-screen absolute text-white ease-in-out duration-100 transition-opacity backdrop-blur-sm backdrop-brightness-50 grid place-items-center z-999 transition-opacity origin-center will-change-opacity"
            classList={{
              "opacity-100": modalStack().length > 0,
              "opacity-0": modalStack().length > 0,
            }}
          >
            <For each={modalStack()}>
              {(modal, index) => {
                const ModalComponent = defaultModals[modal.name].component;
                const noHeader = defaultModals[modal.name].noHeader || false;
                const title = defaultModals[modal.name].title || "";

                return (
                  <Show when={modal.name}>
                    <div
                      style={{ "z-index": `${index() + 1}` }}
                      class="absolute top-1/2 left-1/2 -translate-1/2"
                    >
                      <Dynamic
                        component={ModalComponent}
                        data={modal.data}
                        noHeader={noHeader}
                        title={title}
                      />
                    </div>
                  </Show>
                );
              }}
            </For>
          </div>
        </div>
      </Portal>
    </ModalsContext.Provider>
  );
};

export const useModal = () => {
  return useContext(ModalsContext);
};
