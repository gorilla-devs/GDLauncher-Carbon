import { useLocation, useSearchParams } from "@solidjs/router";
import {
  createContext,
  createSignal,
  For,
  JSX,
  lazy,
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
    preventClose?: boolean;
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
      const newStack = currentStack.slice();

      // Remove the specific modal or the top modal
      let indexToRemove: number;
      if (name) {
        indexToRemove = currentStack.findIndex((modal) => modal.name === name);
      } else {
        indexToRemove = currentStack.length - 1;
      }

      if (indexToRemove >= 0) {
        newStack.splice(indexToRemove, 1);
        const newParams: { [k: string]: string | null } = Object.fromEntries(
          urlSearchParams()
        );

        for (let key in Object.fromEntries(urlSearchParams())) {
          if (key !== `m[${indexToRemove + 1}]`) {
            newParams[`m[${indexToRemove + 1}]`] = null;
          }
        }

        setSearchParams(newParams);
      }

      return newStack;
    });

    if (modalStack().length === 0) {
      const overlay = document.getElementById("overlay") as HTMLElement;
      overlay.style.display = "none";
    }
  };

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay") as HTMLElement;
      overlay.style.display = "flex";
      overlay.style.opacity = "0"; // Set initial opacity to 0
      setTimeout(() => (overlay.style.opacity = "1"), 10); // Transition to opacity 1
      setModalStack((currentStack) => [
        ...currentStack,
        { name: modal.name, data },
      ]);

      // Update URL params
      if (modal.url) {
        const url = new URLSearchParams(modal.url);

        url.append(`m[${modalStack().length}]`, modal.name);

        const decodedParamString = decodeURIComponent(url.toString());
        navigate(decodedParamString.replace("=&", "?"));
      } else {
        setSearchParams({
          [`m[${modalStack().length}]`]: modal.name,
        });
      }
    },
    closeModal,
  };

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <div class="w-screen h-screen">
          <For each={modalStack()}>
            {(modal, index) => {
              const ModalComponent = defaultModals[modal.name].component;
              const noHeader = defaultModals[modal.name].noHeader || false;
              const title = defaultModals[modal.name].title || "";
              const preventClose = defaultModals[modal.name].preventClose;

              return (
                <>
                  <div
                    class="absolute bottom-0 top-0 left-0 flex justify-center items-center z-999"
                    onClick={() => {
                      if (!preventClose) {
                        closeModal();
                      }
                    }}
                    classList={{
                      "right-0": location.pathname === "/",
                      "right-[440px]": location.pathname !== "/",
                    }}
                  >
                    <div
                      style={{ "z-index": `${index() + 1}` }}
                      class="duration-100 ease-in-out animate-enterScaleIn"
                    >
                      <Dynamic
                        component={ModalComponent}
                        data={modal.data}
                        noHeader={noHeader}
                        title={title}
                      />
                    </div>
                  </div>
                  <div
                    class="h-screen w-screen absolute text-white transition-all duration-100 ease-in-out backdrop-blur-sm grid place-items-center z-99 origin-center"
                    onClick={() => {
                      if (!preventClose) {
                        closeModal();
                      }
                    }}
                  >
                    <div class="h-screen w-screen bg-darkSlate-900 opacity-80" />
                  </div>
                </>
              );
            }}
          </For>
        </div>
      </Portal>
    </ModalsContext.Provider>
  );
};

export const useModal = () => {
  return useContext(ModalsContext);
};
