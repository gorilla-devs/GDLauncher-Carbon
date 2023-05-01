import { Show, mergeProps } from "solid-js";

type Props = {
  name: string;
  img: string | undefined | null;
  type?: "fixed" | "default";
  onClose?: () => void;
};

export const Tag = (props: Props) => {
  const mergedProps = mergeProps({ type: "default" }, props);

  return (
    <div
      class="flex gap-2 items-center px-3 py-2 bg-darkSlate-700 rounded-md max-h-8 h-full box-border"
      classList={{
        "bg-darkSlate-700": mergedProps.type === "default",
        "bg-darkSlate-900": mergedProps.type === "fixed",
      }}
    >
      <Show when={props.img}>
        <img
          class="w-4 h-4"
          src={props.img as string}
          alt={`icon_${props.name}`}
        />
      </Show>
      <p class="m-0 text-darkSlate-100 whitespace-nowrap">{props.name}</p>
      <Show when={mergedProps.type === "default"}>
        <div
          class="i-ri:close-fill text-lg text-darkSlate-200 cursor-pointer"
          onClick={() => props.onClose?.()}
        />
      </Show>
    </div>
  );
};
