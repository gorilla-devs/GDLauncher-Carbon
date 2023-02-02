import { Match, Switch, mergeProps } from "solid-js";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: string;
  isLoading?: boolean;
  percentage?: number;
  version: string;
  variant?: Variant;
  onClick?: (_e: MouseEvent) => void;
};

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  );

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <div
          class="flex flex-col justify-center items-start cursor-pointer snap-start"
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
          class="h-14 w-full flex items-center gap-4 hover:bg-primary bg-opacity-10 px-3"
          onClick={(e) => props?.onClick?.(e)}
        >
          <div class="h-10 w-10 bg-green-600 rounded-lg" />
          <div class="flex flex-col">
            <h4
              class="m-0"
              classList={{
                "text-shade-0": mergedProps.isLoading,
                "text-white": !mergedProps.isLoading,
              }}
            >
              {props.title}
            </h4>
            <div class="flex justify-between text-shade-0">
              <p class="m-0">{props.modloader}</p>
              <p class="m-0">{props.version}</p>
            </div>
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
