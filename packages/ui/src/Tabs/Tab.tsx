import {
  Match,
  Switch,
  createSignal,
  JSXElement,
  onMount,
  onCleanup,
  untrack,
  JSX,
  splitProps,
} from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props extends JSX.HTMLAttributes<HTMLDivElement> {
  children: JSXElement | string | number;
  onClick?: () => void;
}

const Tab = (_props: Props) => {
  const [onclick, props] = splitProps(_props, ["onClick"]);
  const [index, setIndex] = createSignal(-1);
  let ref: HTMLDivElement;

  const tabsContext = useTabsContext();

  let observer: ResizeObserver;

  onMount(() => {
    if (tabsContext) {
      setIndex(tabsContext.registerTab(ref!));
    }

    observer = new ResizeObserver((args) => {
      untrack(() => {
        const cr = args[0].target as HTMLDivElement;
        tabsContext!.registerTab(cr, index());
      });
    });
    observer.observe(ref!);
  });

  onCleanup(() => {
    observer?.disconnect();
  });

  return (
    <div
      class="cursor-pointer"
      classList={{
        "w-full": tabsContext?.variant === "block",
      }}
      ref={(el) => {
        ref = el;
      }}
      onClick={() => {
        if (onclick.onClick) onclick.onClick();
        tabsContext?.setSelectedIndex(index());
      }}
      {...props}
    >
      <Switch>
        <Match when={tabsContext?.variant === "underline"}>
          <div
            class="cursor-pointer bg-shade-8 font-500 capitalize"
            classList={{
              "py-5": tabsContext?.orientation === "horizontal",
              "px-4": tabsContext?.orientation === "vertical",
              "text-white": tabsContext?.isSelectedIndex(index()),
              "text-shade-0": !tabsContext?.isSelectedIndex(index()),
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant === "block"}>
          <div
            class="flex gap-1 justify-center items-center bg-shade-8 flex-1 h-full cursor-pointer rounded-xl font-500 capitalize box-border"
            classList={{
              "py-5 px-2": tabsContext?.orientation === "horizontal",
              "px-4 py-2": tabsContext?.orientation === "vertical",
              "text-white": tabsContext?.isSelectedIndex(index()),
              "text-shade-0": !tabsContext?.isSelectedIndex(index()),
            }}
          >
            {props.children}
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export { Tab };
