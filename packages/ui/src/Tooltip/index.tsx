import { JSX, createSignal, Show, createEffect, onCleanup } from "solid-js";
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
  color?: string;
  noTip?: boolean;
  noPadding?: boolean;
  delay?: number;
  opened?: boolean;
};

const Tooltip = (props: Props) => {
  const [tooltipOpened, setTooltipOpened] = createSignal(false);
  const [elementRef, setElementRef] = createSignal<
    HTMLDivElement | undefined
  >();
  const [toolTipRef, setToolTipRef] = createSignal<
    HTMLDivElement | undefined
  >();

  const position = useFloating(elementRef, toolTipRef, {
    placement: props.placement || "top",
    middleware: [offset(10), flip(), shift(), hide(), size()],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
  });

  createEffect(() => {
    if (position.middlewareData.hide?.referenceHidden) setTooltipOpened(false);
  });

  const [triangleStart, setTriangleStart] = createSignal({ x: 0, y: 0 });
  const [triangleEnd, setTriangleEnd] = createSignal({ x: 0, y: 0 });

  let hoverTimeout: ReturnType<typeof setTimeout> | undefined;
  let closeTimeout: ReturnType<typeof setTimeout> | undefined;

  function handleMouseMove(e: MouseEvent) {
    const x = e.clientX;
    const y = e.clientY;

    if (!toolTipRef() || !elementRef()) return;

    const toolTipRect = (
      toolTipRef() as HTMLDivElement
    ).getBoundingClientRect();
    const elementRect = (
      elementRef() as HTMLDivElement
    ).getBoundingClientRect();

    // if the mouse is over the tooltip content or the tooltip reference, return
    if (
      (x >= toolTipRect.left &&
        x <= toolTipRect.right &&
        y >= toolTipRect.top &&
        y <= toolTipRect.bottom) ||
      (x >= elementRect.left &&
        x <= elementRect.right &&
        y >= elementRect.top &&
        y <= elementRect.bottom)
    ) {
      if (typeof closeTimeout !== "undefined") {
        clearTimeout(closeTimeout);
        closeTimeout = undefined;
      }
      return;
    }

    // check if the mouse is outside of the safe triangle
    if (
      (x < triangleStart().x && x < triangleEnd().x) ||
      (x > triangleStart().x && x > triangleEnd().x)
    ) {
      // delay closing the tooltip to allow time for the mouse to move to the tooltip content
      closeTimeout = setTimeout(() => {
        setTooltipOpened(false);
        closeTimeout = undefined;
      }, 400);
    }
  }

  window.addEventListener("mousemove", handleMouseMove);

  onCleanup(() => {
    window.removeEventListener("mousemove", handleMouseMove);
    if (typeof closeTimeout !== "undefined") {
      clearTimeout(closeTimeout);
    }
    if (typeof hoverTimeout !== "undefined") {
      clearTimeout(hoverTimeout);
    }
  });

  return (
    <>
      <Show when={props.opened || tooltipOpened()}>
        <Portal>
          <div
            ref={(el) => setToolTipRef(el)}
            class={`absolute rounded-lg ${props.color || ""}`}
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
        onMouseEnter={(e) => {
          setTriangleStart({ x: e.clientX, y: e.clientY });
          hoverTimeout = setTimeout(() => {
            setTooltipOpened(true);
          }, props.delay ?? 400);
        }}
        onMouseLeave={(e) => {
          setTriangleEnd({ x: e.clientX, y: e.clientY });
        }}
      >
        {props.children}
      </div>
    </>
  );
};

export { Tooltip };
