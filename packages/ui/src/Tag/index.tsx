import { Show, mergeProps, JSX } from "solid-js";

type Props = {
  name?: string;
  img?: string | JSX.Element | undefined | null;
  type?: "fixed" | "default";
  size?: "medium" | "small";
  onClose?: (_name: string) => void;
};

export const Tag = (props: Props) => {
  const mergedProps = mergeProps({ type: "default" }, props);

  return (
    <div
      class="flex gap-2 items-center bg-darkSlate-700 rounded-md max-h-8 h-full box-border select-none"
      classList={{
        "bg-darkSlate-700": mergedProps.type === "default",
        "bg-darkSlate-900": mergedProps.type === "fixed",
        "px-3 py-2": props.size === "medium" || !props.size,
        "px-2 py-1": props.size === "small",
      }}
    >
      <Show when={props.img && typeof props.img === "string"}>
        <img
          class="w-4 h-4"
          src={props.img as string}
          alt={`icon_${props.name}`}
        />
      </Show>
      <Show when={props.img && typeof props.img !== "string"}>{props.img}</Show>
      <Show when={props.name}>
        <p class="m-0 text-darkSlate-100 whitespace-nowrap">{props.name}</p>
      </Show>
      <Show when={mergedProps.type === "default"}>
        <div
          class="i-ri:close-fill text-lg text-darkSlate-200 cursor-pointer"
          onClick={() => {
            if (props.name) props.onClose?.(props.name);
          }}
        />
      </Show>
    </div>
  );
};
