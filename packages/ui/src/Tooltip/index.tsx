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
            class="absolute bg-darkSlate-900 rounded-lg px-2 py-1"
            style={{
              position: "absolute",
              top: `${position.y ?? 0}px`,
              left: `${position.x ?? 0}px`,
            }}
          >
            <div class="relative z-20">{props.content}</div>
            <div class="absolute left-1/2 -translate-x-1/2 -bottom-1 w-4 h-4 rotate-45 bg-darkSlate-900 z-10" />
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
