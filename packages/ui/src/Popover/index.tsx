import {
  JSX,
  createSignal,
  Show,
  createEffect,
  onMount,
  onCleanup,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import {
  offset,
  flip,
  shift,
  autoUpdate,
  hide,
  size,
  Placement,
} from "@floating-ui/dom";

type Props = {
  children: JSX.Element;
  content: JSX.Element | string | number;
  placement?: Placement;
  hoverable?: string;
  color?: string;
  noTip?: boolean;
  noPadding?: boolean;
  opened?: boolean;
};

const Popover = (props: Props) => {
  const [PopoverOpened, setPopoverOpened] = createSignal(false);
  const [elementRef, setElementRef] = createSignal<
    HTMLDivElement | undefined
  >();
  const [PopoverRef, setPopoverRef] = createSignal<
    HTMLDivElement | undefined
  >();
  // const [isSubMenuOpen, setSubMenuOpen] = createSignal(false);
  const [timer, setTimer] = createSignal<ReturnType<typeof setTimeout> | null>(
    null
  );
  const [lastPos, setLastPos] = createSignal({ x: 0, y: 0 });

  const trackMouse = (e: MouseEvent) => {
    setLastPos({ x: e.clientX, y: e.clientY });
  };

  const startTimer = () => {
    if (!PopoverRef()) return;
    setTimer(
      setTimeout(() => {
        const menuRect = (
          PopoverRef() as HTMLDivElement
        ).getBoundingClientRect();

        // check if the last mouse position is within the safe triangle
        if (lastPos().x > menuRect.right && lastPos().y > menuRect.top) {
          return;
        }

        setPopoverOpened(false);
      }, 300)
    );
  };

  const stopTimer = () => {
    if (timer()) {
      clearTimeout(timer() as ReturnType<typeof setTimeout>);
    }
    setTimer(null);
  };

  onMount(() => {
    window.addEventListener("mousemove", trackMouse);
  });

  onCleanup(() => {
    window.removeEventListener("mousemove", trackMouse);
  });

  const position = useFloating(elementRef, PopoverRef, {
    placement: props.placement || "top",
    middleware: [offset(10), flip(), shift(), hide(), size()],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
  });

  createEffect(() => {
    if (position.middlewareData.hide?.referenceHidden) setPopoverOpened(false);
  });

  return (
    <>
      <Show when={props.opened || PopoverOpened()}>
        <Portal>
          <div
            onMouseEnter={stopTimer}
            onMouseLeave={startTimer}
            ref={(el) => setPopoverRef(el)}
            class={`absolute rounded-lg z-[100] ${props.color || ""}`}
            style={{
              position: "absolute",
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
            classList={{
              "bg-darkSlate-900": !props.color,
              "px-2 py-1": !props.noPadding,
            }}
          >
            <div class="relative z-20">{props.content}</div>
            <Show when={!props.noTip}>
              <div
                class={`absolute w-4 h-4 rotate-45 z-10 ${props.color || ""}`}
                classList={{
                  "bg-darkSlate-900": !props.color,
                  "left-1/2 -translate-x-1/2 -bottom-1":
                    props.placement?.includes("top") || !props.placement,
                  "top-1/2 -translate-y-1/2 -left-1":
                    props.placement?.includes("right"),
                }}
              />
            </Show>
          </div>
        </Portal>
      </Show>

      <div
        ref={(el) => setElementRef(el)}
        onMouseEnter={() => setPopoverOpened(true)}
        onMouseLeave={startTimer}
      >
        {props.children}
      </div>
    </>
  );
};

export { Popover };
