import {
  JSX,
  createSignal,
  Show,
  createEffect,
  onMount,
  onCleanup,
  mergeProps,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import {
  offset,
  shift,
  autoUpdate,
  hide,
  size,
  Placement,
  autoPlacement,
} from "@floating-ui/dom";

type Props = {
  children: JSX.Element;
  content: JSX.Element | string | number;
  trigger?: "click" | "hover";
  placement?: Placement;
  color?: string;
  noTip?: boolean;
  noPadding?: boolean;
  opened?: boolean;
  onOpen?: () => void;
  onClose?: () => void;
};

type Point = { x: number; y: number };

function isPointInTriangle(
  pt: Point,
  v1: Point,
  v2: Point,
  v3: Point
): boolean {
  const dX = pt.x - v3.x;
  const dY = pt.y - v3.y;
  const dX21 = v3.x - v2.x;
  const dY12 = v2.y - v3.y;
  const D = dY12 * (v1.x - v3.x) + dX21 * (v1.y - v3.y);
  const s = dY12 * dX + dX21 * dY;
  const t = (v3.y - v1.y) * dX + (v1.x - v3.x) * dY;
  if (D < 0) return s <= 0 && t <= 0 && s + t >= D;
  return s >= 0 && t >= 0 && s + t <= D;
}

const Popover = (_props: Props) => {
  const props: Props = mergeProps({ trigger: "hover" as "hover" }, _props);

  const [isHoveringCard, setSsHoveringCard] = createSignal(false);
  const [PopoverOpened, setPopoverOpened] = createSignal(false);
  const [elementRef, setElementRef] = createSignal<HTMLDivElement>();
  const [PopoverRef, setPopoverRef] = createSignal<HTMLDivElement>();
  const [triangleStart, setTriangleStart] = createSignal({ x: 0, y: 0 });
  const [openTimer, setOpenTimer] =
    createSignal<ReturnType<typeof setTimeout>>();

  const open = () => {
    if (props.trigger === "click") {
      window.addEventListener("click", onClickWindow, true);
    }
    setPopoverOpened(true);
    props.onOpen?.();
  };

  const close = () => {
    if (props.trigger === "click") {
      window.removeEventListener("click", onClickWindow, true);
    }
    setPopoverOpened(false);
    props.onClose?.();
  };

  const menuRect = () => {
    if (!PopoverRef()) return;
    const popover = PopoverRef() as HTMLDivElement;
    return popover && popover.offsetWidth > 0 && popover.offsetHeight > 0
      ? popover.getBoundingClientRect()
      : undefined;
  };

  const trackMouse = (e: MouseEvent) => {
    if (!menuRect()) return;
    const b = {
      x: (menuRect() as DOMRect).left,
      y: (menuRect() as DOMRect).top,
    };
    const c = {
      x: (menuRect() as DOMRect).left,
      y: (menuRect() as DOMRect).bottom,
    };

    const a = triangleStart();

    if (
      !isPointInTriangle({ x: e.clientX, y: e.clientY }, a, b, c) &&
      !isHoveringCard()
    ) {
      close();
    } else {
      open();
    }
  };

  let debounceTimeout: ReturnType<typeof setTimeout> | null = null;

  const debouncedTrackMouse = (e: MouseEvent) => {
    if (debounceTimeout) clearTimeout(debounceTimeout);

    debounceTimeout = setTimeout(() => {
      trackMouse(e);
    }, 50);
  };

  const onClickWindow = (event: MouseEvent) => {
    if (PopoverRef() && !PopoverRef()!.contains(event.target as any)) {
      event.stopPropagation();
      event.preventDefault();
      close();
    }
  };

  onMount(() => {
    if (props.trigger === "hover") {
      window.addEventListener("mousemove", debouncedTrackMouse);
    }
  });

  onCleanup(() => {
    if (props.trigger === "hover") {
      window.removeEventListener("mousemove", debouncedTrackMouse);
    }
  });

  const position = useFloating(elementRef, PopoverRef, {
    placement: props.placement || "top",
    middleware: [
      offset(10),
      shift(),
      hide(),
      size(),
      autoPlacement({
        padding: {
          top: 0,
          right: 200,
        },
      }),
    ],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
  });

  createEffect(() => {
    if (position.middlewareData.hide?.referenceHidden) {
      close();
    }
  });

  return (
    <>
      <Show when={props.opened || PopoverOpened()}>
        <Portal>
          <Show when={props.trigger === "click"}>
            <div class="w-screen h-screen absolute top-0 left-0 backdrop-blur-[2px] z-100" />
          </Show>
          <div
            onMouseEnter={() => {
              setSsHoveringCard(true);
            }}
            onMouseLeave={() => {
              setSsHoveringCard(false);
            }}
            ref={setPopoverRef}
            class={`rounded-lg will-change z-100 ${props.color || ""}`}
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
        ref={setElementRef}
        onMouseEnter={(e) => {
          setTriangleStart({ x: e.clientX, y: e.clientY });
          setSsHoveringCard(true);
          if (props.trigger === "hover") {
            setOpenTimer(
              setTimeout(() => {
                open();
              }, 300)
            );
          }
        }}
        onMouseLeave={() => {
          if (props.trigger === "hover") {
            clearTimeout(openTimer());
          }
          setSsHoveringCard(false);
        }}
        onClick={() => {
          if (props.trigger === "click") {
            if (PopoverOpened()) {
              close();
            } else {
              open();
            }
          }
        }}
      >
        {props.children}
      </div>
    </>
  );
};

export { Popover };
