import { useLocation, useSearchParams } from "@solidjs/router";
import {
  createContext,
  createSignal,
  For,
  JSX,
  lazy,
  useContext
} from "solid-js";
import { Dynamic, Portal } from "solid-js/web";
import { useGDNavigate } from "../NavigationManager";
import adSize from "@/utils/adhelper";

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

const defaultModals = {
  privacyStatement: {
    component: lazy(() => import("./modals/PrivacyStatement")),
    title: "Privacy Statement"
  },
  termsAndConditions: {
    component: lazy(() => import("./modals/TermsAndConditions")),
    title: "Terms and Conditions"
  },
  addJava: {
    component: lazy(() => import("./modals/Java/AddJava")),
    title: "Add java version"
  },
  modDetails: {
    component: lazy(() => import("./modals/ModDetails")),
    title: "Mod Details"
  },
  javaSetup: {
    component: lazy(() => import("./modals/Java/JavaSetup")),
    title: "Java Setup"
  },
  instanceCreation: {
    component: lazy(() => import("./modals/InstanceCreation")),
    title: "New Instance"
  },
  exportInstance: {
    component: lazy(() => import("./modals/InstanceExport")),
    title: "Export Instance"
  },
  modpack_version_update: {
    component: lazy(() => import("./modals/ModPackVersionUpdate")),
    title: "Update Version"
  },
  confirmation: {
    component: lazy(() => import("./modals/Confirmation"))
  },
  notification: {
    component: lazy(() => import("./modals/Notification")),
    title: "Notification"
  },
  confirmInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmInstanceDeletion")),
    title: "Confirm Instance Deletion"
  },
  ConfirmChangeRuntimePath: {
    component: lazy(() => import("./modals/ConfirmChangeRuntimePath")),
    title: "Confirm Change RuntimePath"
  },
  appUpdate: {
    component: lazy(() => import("./modals/AppUpdate")),
    title: "New App Version Available"
  },
  onBoarding: {
    component: lazy(() => import("./modals/OnBoarding")),
    noHeader: true
  },
  whyAreAdsNeeded: {
    component: lazy(() => import("./modals/WhyAreAdsNeeded")),
    title: "Why are ads needed?"
  },
  modsUpdater: {
    component: lazy(() => import("./modals/ModsUpdater")),
    title: "Mods Updater"
  }
};

type ModalName = keyof typeof defaultModals;

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
        const newParams: { [k: string]: string | null } =
          Object.fromEntries(urlSearchParams());

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
        { name: modal.name, data }
      ]);

      // Update URL params
      if (modal.url) {
        const url = new URLSearchParams(modal.url);

        url.append(`m[${modalStack().length}]`, modal.name);

        const decodedParamString = decodeURIComponent(url.toString());
        navigate(decodedParamString.replace("=&", "?"));
      } else {
        setSearchParams({
          [`m[${modalStack().length}]`]: modal.name
        });
      }
    },
    closeModal
  };

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay") as HTMLElement}>
        <div class="w-screen h-screen">
          <For each={modalStack()}>
            {(modal, index) => {
              const ModalComponent = defaultModals[modal.name].component;
              const noHeader =
                (defaultModals as Hash)[modal.name].noHeader || false;
              const title = (defaultModals as Hash)[modal.name].title || "";
              const preventClose = (defaultModals as Hash)[modal.name]
                .preventClose;

              return (
                <div class="h-screen w-screen flex absolute inset-0">
                  <div
                    class="flex h-full items-center relative flex-grow justify-center z-999"
                    onMouseDown={() => {
                      if (!preventClose) {
                        closeModal();
                      }
                    }}
                  >
                    <div
                      style={{ "z-index": `${index() + 1}` }}
                      onMouseDown={(e) => e.stopPropagation()}
                      class="duration-100 ease-in-out animate-enterScaleIn"
                    >
                      <Dynamic
                        component={ModalComponent}
                        data={modal.data}
                        noHeader={noHeader}
                        title={title}
                      />
                    </div>
                    <div class="absolute inset-0 bg-darkSlate-900 backdrop-blur-sm opacity-80" />
                  </div>

                  <div
                    class="h-screen duration-100 ease-in-out text-white transition-all grid place-items-center z-99 origin-center"
                    style={{
                      width: `${adSize.width + 40}px`
                    }}
                    onClick={() => {
                      if (!preventClose) {
                        closeModal();
                      }
                    }}
                  />
                </div>
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
