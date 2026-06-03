import { onMount, createSignal, Show } from "solid-js";
import { detectOS } from "../utils/detectOS";
import type { OS } from "../utils/detectOS";

const OS_ICONS: Record<OS, string> = {
  Windows: "i-simple-icons:windows11",
  MacOS: "i-simple-icons:apple",
  Linux: "i-simple-icons:linux",
}

const OS_URL: Record<OS, string> = {
  Windows: "/download/windows",
  MacOS: "/download/mac",
  Linux: "/download/linux",
}

interface Props {
  label: string;
  /** Optional Tailwind / unocss class string applied to the <a>. Lets CTA
   *  callers reuse this link with their own button styling instead of
   *  forking the component. */
  class?: string;
  /** Optional icon size class (default `w-4 h-4`). */
  iconClass?: string;
  /** When true, the OS icon is rendered before the label instead of after. */
  iconLeft?: boolean;
}

export const DownloadLink = (props: Props) => {
  // Default to Windows on first paint (covers ~70% of visitors and makes the
  // server-rendered link clickable before hydration). `onMount` flips it to
  // the detected OS the moment we're in the browser.
  const [os, setOS] = createSignal<OS>("Windows");

  onMount(() => {
    setOS(detectOS());
  });

  const iconCls = () => props.iconClass ?? "w-4 h-4";
  const linkCls = () => props.class ?? "flex items-center gap-2";

  return (
    <a
      href={OS_URL[os()]}
      class={linkCls()}
      data-astro-prefetch="false"
    >
      <Show when={props.iconLeft}>
        <div class={`${OS_ICONS[os()]} ${iconCls()}`} aria-hidden="true"></div>
      </Show>
      <span>{props.label}</span>
      <Show when={!props.iconLeft}>
        <div class={`${OS_ICONS[os()]} ${iconCls()}`} aria-hidden="true"></div>
      </Show>
    </a>
  );
};
