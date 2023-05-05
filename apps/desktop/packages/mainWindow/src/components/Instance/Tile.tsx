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
          classList={{
            grayscale: props.isLoading,
          }}
        >
          <div class="group relative rounded-2xl h-38 w-38 bg-green-600">
            <div
              class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 duration-100 ease-in-out hidden transition-all"
              classList={{
                "group-hover:flex": !props.isLoading,
              }}
            >
              <div class="h-12 bg-primary-500 rounded-full flex justify-center items-center cursor-pointer w-12">
                <div
                  class="text-white text-2xl i-ri:play-fill"
                  onClick={(e) => e.preventDefault()}
                />
              </div>
            </div>
            <div
              class="absolute top-2 right-2 duration-100 ease-in-out hidden transition-all"
              classList={{
                "group-hover:flex": !props.isLoading,
              }}
            >
              <div class="flex justify-center items-center cursor-pointer h-7 w-7 bg-darkSlate-500 rounded-full">
                <div
                  class="text-white i-ri:more-2-fill text-lg"
                  onClick={(e) => e.preventDefault()}
                />
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
          </div>
          <h4
            class="mt-2 mb-0"
            classList={{
              "text-white": !props.isLoading,
              "text-lightGray-900": props.isLoading,
            }}
          >
            {props.title}
          </h4>
          <div class="flex text-lightGray-900 justify-between">
            <span class="flex gap-2">
              <img class="w-4 h-4" src={getModloaderIcon(props.modloader)} />
              <p class="m-0">{props.modloader}</p>
            </span>
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
            <div class="absolute right-0 ease-in-out transition duration-100 opacity-10 top-0 left-0 bottom-0 bg-primary-500" />
            <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
          </Show>

          <div
            class="absolute gap-2 duration-100 ease-in-out right-5 hidden transition-all"
            classList={{
              "group-hover:flex": !props.isLoading,
            }}
          >
            <div class="flex justify-center items-center cursor-pointer h-7 w-7 bg-darkSlate-500 rounded-full">
              <div
                class="text-white i-ri:more-2-fill text-lg"
                onClick={(e) => e.preventDefault()}
              />
            </div>
            <div class="h-7 w-7 bg-primary-500 rounded-full flex justify-center items-center cursor-pointer">
              <div
                class="text-white text-lg i-ri:play-fill"
                onClick={(e) => e.preventDefault()}
              />
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
            <div class="flex gap-4 text-darkSlate-50">
              <span class="flex gap-2">
                <img class="w-4 h-4" src={getModloaderIcon(props.modloader)} />
                <p class="m-0">{props.modloader}</p>
              </span>
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
