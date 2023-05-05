import { JSX, createSignal, onCleanup, onMount, untrack } from "solid-js";
import { useTabsContext } from "./Tabs";

export interface Props extends JSX.HTMLAttributes<HTMLDivElement> {}

const Spacing = (props: Props) => {
  const [index, setIndex] = createSignal(-1);
  let ref: HTMLDivElement;

  const tabsContext = useTabsContext();

  let observer: ResizeObserver;

  onMount(() => {
    if (tabsContext) {
      setIndex(
        tabsContext.registerTabSpacing({
          ref: ref,
          type: "spacing",
          space: ref.offsetWidth,
        })
      );
    }

    observer = new ResizeObserver((args) => {
      untrack(() => {
        const cr = args[0].target as HTMLDivElement;
        tabsContext!.registerTabSpacing(
          { ref: cr, type: "spacing", space: cr.offsetWidth },
          index()
        );
      });
    });
    observer?.observe(ref!);
  });

  onCleanup(() => {
    observer?.disconnect();
  });

  return (
    <div
      ref={(el) => {
        ref = el;
      }}
      {...props}
    />
  );
};

export { Spacing };
