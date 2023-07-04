import { Rive } from "@rive-app/canvas";
import { createSignal, onCleanup, onMount } from "solid-js";

type Props = {
  src: string;
  onStop?: () => void;
  width?: number;
  height?: number;
};

const RiveAppWapper = (props: Props) => {
  let canvas: HTMLCanvasElement | undefined;

  const [riveRef, setRiveRef] = createSignal<Rive | undefined>();
  onMount(() => {
    if (canvas && props.src) {
      const r = new Rive({
        src: props.src,
        autoplay: true,
        canvas: canvas,
        stateMachines: ["State Machine 1"],
        onLoad: () => {
          r.resizeDrawingSurfaceToCanvas();
          setRiveRef(r);
        },
        onStateChange: (state) => {
          // there is no way to dected the end of an animation in rive.app
          // to achive it, I added another state at the end of the animatio so I can detect it
          // sorry idk how to change the name of the state, so it's ""
          if ((state.data as string[])[0] === "") {
            props?.onStop?.();
          }
        },
      });
    }
  });

  onCleanup(() => {
    if (riveRef()) {
      riveRef()?.stopRendering();
      riveRef()?.cleanup();
    }
  });
  return (
    <canvas
      ref={canvas}
      width={props.width || 600}
      height={props.height || 600}
      style={{
        width: `${props.width || 600}px`,
        height: `${props.height || 600}px`,
      }}
    />
  );
};

export default RiveAppWapper;
