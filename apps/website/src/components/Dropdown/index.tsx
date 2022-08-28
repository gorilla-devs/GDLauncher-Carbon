import { createSignal, For, mergeProps } from "solid-js";
import Caret from "../CaretIcon";

type Props = {
  children?: HTMLElement | string;
  class?: string;
};

enum OS {
  windows = "windows",
  macos = "macos",
  linux = "linux",
}

const urls = {
  [OS.windows]:
    "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-win-setup.exe",
  [OS.macos]:
    "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-mac-setup.dmg",
  [OS.linux]:
    "https://github.com/gorilla-devs/GDLauncher/releases/latest/download/GDLauncher-linux-setup.AppImage",
};

const Dropdown = (props: Props) => {
  const [currentValue, setCurrentValue] = createSignal<OS>(OS.windows);
  const [open, setOpen] = createSignal(false);
  const merged = mergeProps(props);

  // createEffect(() => {
  //   const platform = navigator?.platform;
  // });

  return (
    <div class="relative max-w-[300px] bg-slate-600 rounded-2xl">
      {open() && (
        <div class="absolute top-0 left-0 right-0 bottom-0 bg-slate-600 -z-1 rounded-t-2xl" />
      )}
      <div
        onclick={() => setOpen(!open())}
        class={`flex justify-between items-center font-main text-white font-bold py-4 px-10 rounded-2xl bg-[#2b6cb0] cursor-pointer ${props.class}`}
      >
        <a href={urls[currentValue()]}>Download for {currentValue()}</a>
        <Caret
          class={`ease-linear duration-100 ${
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
                    }}
                    class="py-4 px-10 w-full cursor-pointer hover:bg-slate-700 ease-linear duration-100"
                  >
                    Download for {os}
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
