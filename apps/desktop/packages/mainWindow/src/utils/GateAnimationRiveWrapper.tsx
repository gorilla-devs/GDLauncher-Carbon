import { Rive } from "@rive-app/canvas";
import { createSignal, onCleanup, onMount } from "solid-js";

type Props = {
  src: string;
  width?: number;
  height?: number;
};

const RiveAppWapper = (props: Props) => {
  let canvas: HTMLCanvasElement | undefined;

  const [riveRef, setRiveRef] = createSignal<Rive | undefined>();
  onMount(() => {
    const buttonOpen = document.getElementById("login-btn");
    const buttonLinkBtn = document.getElementById("login-link-btn");

    if (canvas && props.src) {
      const r = new Rive({
        src: props.src,
        autoplay: true,
        canvas: canvas,
        stateMachines: ["gate"],
        onLoad: () => {
          r.resizeDrawingSurfaceToCanvas();
          setRiveRef(r);
          const inputs = r.stateMachineInputs("gate");
          const openGate = inputs.find((i) => i.name === "openGate");

          if (buttonOpen && buttonLinkBtn) {
            buttonOpen.onclick = () => {
              openGate?.fire();
            };

            buttonLinkBtn.onclick = () => {
              openGate?.fire();
            };
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
