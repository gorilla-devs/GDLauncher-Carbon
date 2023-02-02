import { Match, Show, Switch, mergeProps } from "solid-js";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: string;
  selected?: boolean;
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
          class="group relative h-14 w-full flex items-center gap-4 px-3 box-border cursor pointer"
          onClick={(e) => props?.onClick?.(e)}
        >
          <Show when={props.selected}>
            <div class="r:bg-primary opacity-10 transition duration-100 ease-in-out absolute top-0 left-0 right-0 bottom-0" />
            <div class="w-1 absolute right-0 top-0 bottom-0 bg-primary" />
          </Show>

          <div class="absolute right-5 gap-2 hidden group-hover:flex transition-all duration-100 ease-in-out">
            <div class="h-7 w-7 bg-shade-5 rounded-full flex justify-center items-center">
              <div class="i-ri:more-2-fill text-white text-lg" />
            </div>
            <div class="h-7 w-7 bg-primary rounded-full flex justify-center items-center">
              <div class="i-ri:play-fill text-white text-lg" />
            </div>
          </div>

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
