import { createNotification } from "@gd/ui";
import { createSignal } from "solid-js";

interface Props {
  text: string | null | undefined | number;
}

const CopyIcon = (props: Props) => {
  const [clicked, setClicked] = createSignal(false);
  const addNotification = createNotification();

  return (
    <div
      class="i-ri:clipboard-line transition-transform duration-200 hover:scale-120 hover:text-white"
      classList={{
        "animate-scaleBounce": clicked()
      }}
      onClick={() => {
        if (!props.text) return;
        navigator.clipboard.writeText(props.text as string);
        addNotification("Copied to clipboard", "success");
        setClicked(true);
        setTimeout(() => {
          setClicked(false);
        }, 600);
      }}
    />
  );
};

export default CopyIcon;
