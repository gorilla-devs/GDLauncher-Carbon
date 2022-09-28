import { APP_URLS } from "@/constants";
import { UIDictionaryKeys, useTranslations } from "@/i18n/utils";
import { createSignal, For } from "solid-js";
import Caret from "../CaretIcon";

type Props = {
  children?: HTMLElement | string;
  class?: string;
  pathname: string;
  onclick: () => void;
};

type ILabels = {
  [os in OS]: UIDictionaryKeys;
};

enum OS {
  windows = "windows",
  macos = "macos",
  linux = "linux",
}

const urls = {
  [OS.windows]: APP_URLS.download.win,
  [OS.macos]: APP_URLS.download.macOs,
  [OS.linux]: APP_URLS.download.linux,
};

const labels: ILabels = {
  [OS.windows]: "download.windows",
  [OS.macos]: "download.macos",
  [OS.linux]: "download.linux",
};

const Dropdown = (props: Props) => {
  const [currentValue, setCurrentValue] = createSignal<OS>(OS.windows);
  const [open, setOpen] = createSignal(false);
  const t = useTranslations(props.pathname);

  // createEffect(() => {
  // TODO: get default language
  //   const platform = navigator?.platform;
  // });

  return (
    <div class="relative max-w-[300px] bg-slate-600 rounded-2xl">
      {open() && (
        <div class="absolute top-0 left-0 right-0 bottom-0 bg-slate-600 -z-10 rounded-t-2xl" />
      )}
      <div
        onclick={() => setOpen(!open())}
        class={`flex justify-between items-center font-main text-white font-bold py-4 px-10 rounded-2xl bg-[#2b6cb0] cursor-pointer z-10 ${
          props.class || ""
        }`}
      >
        <a href={urls[currentValue()]} onclick={() => props.onclick()}>
          {t(labels[currentValue()])}
        </a>
        <Caret
          class={`ease-linear duration-100 w-4 h-4 ${
            open() ? "rotate-0" : "-rotate-90"
          }`}
        />
      </div>
      {open() && (
        <div class="absolute flex flex-col justify-between items-center bg-slate-600 rounded-b-xl max-w-[300px] w-full overflow-hidden">
          <ul class="w-full">
            <For each={Object.values(OS).filter((os) => os !== currentValue())}>
              {(os) => (
                <a href={urls[os]}>
                  <li
                    onclick={() => {
                      setCurrentValue(os);
                      setOpen(!open());
                      props.onclick();
                    }}
                    class="py-4 px-10 w-full cursor-pointer hover:bg-slate-700 ease-linear duration-100"
                  >
                    {t(labels[os])}
                  </li>
                </a>
              )}
            </For>
          </ul>
        </div>
      )}
    </div>
  );
};

export default Dropdown;
