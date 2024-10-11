import { Trans } from "@gd/i18n";
import { createEffect, createSignal } from "solid-js";

interface Props {
  value?: string;
  icon?: string;
  onClick?: () => void;
  handleRefresh?: () => void;
  expired?: boolean;
  id?: string;
}

export const DeviceCode = (props: Props) => {
  let animateDivRef: HTMLDivElement | undefined = undefined;
  const [copied, setCopied] = createSignal(false);
  const [refreshing, setRefreshing] = createSignal(false);

  function animateCopied() {
    setCopied(true);
    if (animateDivRef) {
      animateDivRef.animate([{ opacity: 0 }, { opacity: 100 }], {
        duration: 150,
        easing: "ease-in-out",
        fill: "forwards"
      });

      setTimeout(() => {
        animateDivRef?.animate([{ opacity: 100 }, { opacity: 0 }], {
          duration: 150,
          easing: "ease-in-out",
          fill: "forwards"
        });

        setTimeout(() => {
          setCopied(false);
        }, 150);
      }, 1000);
    }
  }

  createEffect(() => {
    if (refreshing() && !props.expired) {
      setRefreshing(false);
    }
  });

  return (
    <div class="relative h-13 flex justify-center items-center text-lightSlate-50 font-bold gap-2 rounded-md bg-darkSlate-900 opacity-100 w-54 font-ubuntu border-solid border-1 border-lightSlate-900 overflow-hidden">
      <div
        ref={animateDivRef}
        class="z-1 absolute w-full h-full top-0 left-0 bg-green-500 flex justify-center items-center opacity-0 text-sm"
        classList={{
          "translate-x-full": !copied(),
          "translate-x-0": copied()
        }}
      >
        <i class="i-ri:check-fill w-4 h-4 mr-2" />
        <Trans key="instance.copied_to_clipboard" />
      </div>

      <span
        class="text-2xl font-normal"
        classList={{
          "text-lightSlate-50": !props.expired,
          "text-darkSlate-50": props.expired
        }}
      >
        {props.value}
      </span>
      <span
        id={props.id}
        class="transition-color duration-100 ease-in-out hover:bg-lightSlate-50"
        classList={{
          "i-ri:file-copy-fill": !props.expired,
          "i-ri:refresh-line": props.expired,
          "animate-spin": refreshing(),
          "text-lightSlate-50": props.expired,
          "text-darkSlate-50": !props.expired
        }}
        onClick={async () => {
          if (props.expired) {
            if (props.handleRefresh) {
              setRefreshing(true);
              try {
                await props.handleRefresh();
              } catch (e) {
                console.error(e);
              }
            }
          } else {
            window.copyToClipboard(props.value || "");
            animateCopied();

            if (props?.onClick && !props.expired) {
              props.onClick();
            }
          }
        }}
      />
    </div>
  );
};
