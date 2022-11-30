import type { ComponentProps, JSX } from "solid-js";

interface Props {
  icon: (props: ComponentProps<"svg">) => JSX.Element;
  class?: string;
};

export default function Icon(props: Props): HTMLElement {
  return (<props.icon class={props.class || ""} />) as HTMLElement;
}
