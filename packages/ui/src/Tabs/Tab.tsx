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
import { cva, type VariantProps } from "class-variance-authority";

const tabStyles = cva(
  "flex justify-center items-center text-center transition-colors",
  {
    variants: {
      variant: {
        block: "w-full",
        traditional: "w-auto",
        underline: "",
      },
      orientation: {
        horizontal: "",
        vertical: "",
      },
      isSelected: {
        true: "text-lightSlate-50",
        false: "text-lightSlate-800",
      },
      noPadding: {
        true: "flex flex-col justify-center",
        false: "",
      },
    },
    compoundVariants: [
      {
        variant: "underline",
        orientation: "horizontal",
        class: "h-full",
      },
    ],
    defaultVariants: {
      variant: "block",
      orientation: "horizontal",
      isSelected: false,
      noPadding: false,
    },
  }
);

const tabContentStyles = cva("font-500 capitalize flex items-center", {
  variants: {
    variant: {
      underline: "",
      block: "flex-1 h-full cursor-pointer rounded-xl",
      traditional: "flex-1 h-full rounded-t-xl",
    },
    orientation: {
      horizontal: "",
      vertical: "",
    },
    isSelected: {
      true: "",
      false: "",
    },
    centerContent: {
      true: "justify-center",
      false: "",
    },
  },
  compoundVariants: [
    {
      variant: "block",
      isSelected: true,
      class: "text-lightSlate-50 bg-darkSlate-800",
    },
    {
      variant: "block",
      isSelected: false,
      class: "text-darkSlate-50",
    },
    {
      variant: "traditional",
      isSelected: true,
      class: "bg-darkSlate-700",
    },
  ],
});

interface Props
  extends JSX.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof tabStyles> {
  children: JSXElement | string | number;
  onClick?: () => void;
  ignored?: boolean;
  noPointer?: boolean;
  noPadding?: boolean;
  centerContent?: boolean;
}

const Tab = (_props: Props) => {
  const [{ onClick, class: classProp }, props] = splitProps(_props, [
    "onClick",
    "class",
  ]);
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
      class={tabStyles({
        variant: tabsContext?.variant(),
        orientation: tabsContext?.orientation(),
        isSelected: tabsContext?.isSelectedIndex(index()),
        noPadding: props.noPadding,
        class: `bg-darkSlate-800 hover:text-lightSlate-50 ${classProp}`,
      })}
      ref={(el) => {
        ref = el;
      }}
      onClick={() => {
        if (onClick) onClick();
        if (!props.ignored) tabsContext?.setSelectedIndex(index());
      }}
      {...props}
    >
      <Switch>
        <Match when={tabsContext?.variant() === "underline"}>
          <div
            class={tabContentStyles({
              variant: "underline",
              orientation: tabsContext?.orientation(),
              centerContent: props.centerContent,
              class: `${tabsContext?.paddingX?.() || ""} ${
                tabsContext?.paddingY?.() || ""
              }`,
            })}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "block"}>
          <div
            class={tabContentStyles({
              variant: "block",
              orientation: tabsContext?.orientation(),
              isSelected: tabsContext?.isSelectedIndex(index()),
              class: `${tabsContext?.paddingX?.() || ""} ${
                tabsContext?.paddingY?.() || ""
              }`,
            })}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.variant() === "traditional"}>
          <div
            class={tabContentStyles({
              variant: "traditional",
              isSelected: tabsContext?.isSelectedIndex(index()),
              class: `${tabsContext?.paddingX?.() || ""} ${
                tabsContext?.paddingY?.() || ""
              }`,
            })}
          >
            {props.children}
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export { Tab };
