import { Match, Switch, mergeProps } from "solid-js";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: string;
  version: string;
  variant?: Variant;
  onClick?: (_e: MouseEvent) => void;
};

const Tile = (props: Props) => {
  const mergedProps = mergeProps({ variant: "default" }, props);

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <div
          class="instance-tile flex flex-col justify-center items-start cursor-pointer snap-start"
          onClick={(e) => props?.onClick?.(e)}
        >
          <div class="h-38 w-38 bg-green-600 rounded-2xl" />
          <h4 class="my-2">{props.title}</h4>
          <div class="flex justify-between text-shade-0">
            <p class="m-0">{props.modloader}</p>
            <p class="m-0">{props.version}</p>
          </div>
        </div>
      </Match>
      <Match when={mergedProps.variant === "sidebar"}>
        <div
          class="instance-tile flex flex-col justify-center items-start cursor-pointer snap-start"
          onClick={(e) => props?.onClick?.(e)}
        >
          <div class="h-38 w-38 bg-green-600 rounded-2xl" />
          <h4 class="my-2">{props.title}</h4>
          <div class="flex justify-between text-shade-0">
            <p class="m-0">{props.modloader}</p>
            <p class="m-0">{props.version}</p>
          </div>
        </div>
      </Match>
      <Match when={mergedProps.variant === "sidebar-small"}>
        <div onClick={(e) => props?.onClick?.(e)}>
          <div class="h-10 w-10 bg-green-600 rounded-lg" />
        </div>
      </Match>
    </Switch>
  );
};

export default Tile;
