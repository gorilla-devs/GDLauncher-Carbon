import { JSX, createSignal, Show, createEffect } from "solid-js";
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
    middleware: [offset(5), flip(), shift(), hide(), size()],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
  });

  createEffect(() => {
    if (position.middlewareData.hide?.referenceHidden) setTooltipOpened(false);
  });

  return (
    <>
      <Show when={tooltipOpened()}>
        <Portal>
          <div
            ref={(el) => setToolTipRef(el)}
            class={`absolute rounded-lg px-2 py-1 ${props.color || ""}`}
            style={{
              position: "absolute",
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
            classList={{
              "bg-darkSlate-900": !props.color,
            }}
          >
            <div class="relative z-20">{props.content}</div>
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
          </div>
        </Portal>
      </Show>

      <div
        ref={(el) => setElementRef(el)}
        onMouseOver={() => {
          setTooltipOpened(true);
        }}
        onMouseOut={() => {
          setTooltipOpened(false);
        }}
      >
        {props.children}
      </div>
    </>
  );
};

export { Tooltip };
