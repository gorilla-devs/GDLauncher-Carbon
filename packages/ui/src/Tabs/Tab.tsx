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
  ignored?: boolean;
  noPointer?: boolean;
  noPadding?: boolean;
  centerContent?: boolean;
}

const Tab = (_props: Props) => {
  const [onclick, props] = splitProps(_props, ["onClick"]);
  const [index, setIndex] = createSignal(-1);
  let ref: HTMLDivElement;

  const tabsContext = useTabsContext();

  let observer: ResizeObserver;

  onMount(() => {
    if (tabsContext) {
      setIndex(
        tabsContext.registerTab({
          ref: ref,
          type: "tab",
          ignored: props.ignored,
        })
      );
    }

    observer = new ResizeObserver((args) => {
      untrack(() => {
        const cr = args[0].target as HTMLDivElement;
        tabsContext!.registerTab(
          {
            ref: cr,
            type: "tab",
            ignored: props.ignored,
          },
          index()
        );
      });
    });
    observer?.observe(ref!);
  });

  onCleanup(() => {
    tabsContext?.clearTabs();
    observer?.disconnect();
  });

  return (
    <div
      classList={{
        "w-full": tabsContext?.variant() === "block",
        "w-auto": tabsContext?.variant() === "traditional",
        "h-full":
          tabsContext?.variant() === "underline" &&
          tabsContext?.orientation() === "horizontal",
        "cursor-pointer": !props.noPointer,
        "flex flex-col justify-center": props.noPadding,
        "text-lightSlate-50": tabsContext?.isSelectedIndex(index()),
        "text-lightSlate-800": !tabsContext?.isSelectedIndex(index()),
      }}
      class="bg-darkSlate-800 hover:text-lightSlate-50 flex items-center"
      ref={(el) => {
        ref = el;
      }}
      onClick={() => {
        if (onclick.onClick) onclick.onClick();
        if (!props.ignored) tabsContext?.setSelectedIndex(index());
      }}
      {...props}
    >
      <Switch>
        <Match when={tabsContext?.variant() === "underline"}>
          <div
            class={`hover:text-lightSlate-50 transition-colors font-500 capitalize flex items-center h-full ${
              tabsContext?.paddingX?.() || ""
            } ${tabsContext?.paddingY?.() || ""}`}
            classList={{
              "border-box": tabsContext?.orientation() === "horizontal",
              "py-2":
                tabsContext?.orientation() === "vertical" &&
                !tabsContext?.paddingY?.() &&
                !props.noPadding,
              "px-5":
                tabsContext?.orientation() === "vertical" &&
                !tabsContext?.paddingX?.(),
              "justify-center": props.centerContent,
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "block"}>
          <div
            class={`flex gap-1 hover:text-lightSlate-50 justify-center items-center flex-1 h-full cursor-pointer rounded-xl font-500 capitalize box-border ${
              tabsContext?.paddingX?.() || ""
            } ${tabsContext?.paddingY?.() || ""}`}
            classList={{
              "py-5":
                tabsContext?.orientation() === "horizontal" &&
                !tabsContext?.paddingY?.(),
              "px-2":
                tabsContext?.orientation() === "horizontal" &&
                !tabsContext?.paddingX?.(),
              "px-4":
                tabsContext?.orientation() === "vertical" &&
                !tabsContext?.paddingX?.(),
              "py-2":
                tabsContext?.orientation() === "vertical" &&
                !tabsContext?.paddingY?.(),
              "text-lightSlate-50 bg-darkSlate-800":
                tabsContext?.isSelectedIndex(index()),
              "text-darkSlate-50": !tabsContext?.isSelectedIndex(index()),
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "traditional"}>
          <div
            class={`flex gap-1 hover:text-lightSlate-50 justify-center items-center bg-darkSlate-800 flex-1 h-full font-500 capitalize box-border rounded-t-xl ${
              tabsContext?.paddingX?.() || ""
            } ${tabsContext?.paddingY?.() || ""}`}
            classList={{
              "px-2": !tabsContext?.paddingX?.(),
              "text-lightSlate-50": tabsContext?.isSelectedIndex(index()),
              "text-lightSlate-800": !tabsContext?.isSelectedIndex(index()),
              "bg-darkSlate-700": tabsContext?.isSelectedIndex(index()),
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
