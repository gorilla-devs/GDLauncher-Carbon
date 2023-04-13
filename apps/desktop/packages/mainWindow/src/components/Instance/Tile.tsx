import { ModloaderType, getModloaderIcon } from "@/utils/sidebar";
import { Match, Show, Switch, mergeProps } from "solid-js";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: ModloaderType;
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
          class="flex justify-center cursor-pointer flex-col items-start snap-start"
          onClick={(e) => props?.onClick?.(e)}
        >
          <div class="rounded-2xl h-38 w-38 bg-green-600" />
          <h4 class="my-2">{props.title}</h4>
          <div class="flex text-darkSlate-50 justify-between">
            <p class="m-0">{props.modloader}</p>
            <p class="m-0">{props.version}</p>
          </div>
        </div>
      </Match>
      <Match when={mergedProps.variant === "sidebar"}>
        <div
          class="relative w-full flex items-center gap-4 box-border group h-14 px-3 cursor pointer"
          onClick={(e) => props?.onClick?.(e)}
        >
          <Show when={props.selected && !props.isLoading}>
            <div class="absolute right-0 duration-100 ease-in-out bg-primary opacity-10 transition top-0 left-0 bottom-0" />
            <div class="absolute right-0 top-0 bottom-0 bg-primary w-1" />
          </Show>

          <div class="absolute gap-2 duration-100 ease-in-out right-5 hidden group-hover:flex transition-all">
            <div class="flex justify-center items-center cursor-pointer h-7 w-7 bg-darkSlate-500 rounded-full">
              <div class="text-white i-ri:more-2-fill text-lg" />
            </div>
            <div class="h-7 w-7 bg-primary rounded-full flex justify-center items-center cursor-pointer">
              <div class="text-white text-lg i-ri:play-fill" />
            </div>
          </div>

          <Show when={props.isLoading && props.percentage !== undefined}>
            <div
              class="absolute left-0 top-0 bottom-0 opacity-10 bg-white"
              style={{
                width: `${props.percentage}%`,
              }}
            />
          </Show>
          <div
            class="h-10 bg-green-600 rounded-lg w-10"
            classList={{
              grayscale: props.isLoading,
            }}
          />
          <div class="flex flex-col">
            <h4
              class="m-0"
              classList={{
                "text-darkSlate-50": mergedProps.isLoading,
                "text-white": !mergedProps.isLoading,
              }}
            >
              {props.title}
            </h4>
            <div class="flex justify-between text-darkSlate-50">
              <img class="w-4 h-4" src={getModloaderIcon(props.modloader)} />
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
