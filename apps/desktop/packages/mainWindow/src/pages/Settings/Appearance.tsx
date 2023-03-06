import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { Show } from "solid-js";
import fetchData from "./settings.appearance.data";
import LoadingError from "@/components/LoadingError";

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const themeName = () => routeData.data.data || "default";

  let mutation = rspc.createMutation(["app.setTheme"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["app.getTheme", null], newTheme);
    },
  });

  return (
    <LoadingError routeData={routeData}>
      <div class="bg-shade-8 w-full h-auto flex flex-col py-5 px-6 box-border">
        <h2 class="m-0 mb-7 text-4">
          <Trans
            key="appearance"
            options={{
              defaultValue: "Appearance",
            }}
          />
        </h2>
        <div class="flex justify-between w-full border-box max-w-[35rem]">
          <div
            class="flex flex-col flex justify-center items-center cursor-pointer w-42 p-1 bg-[#15181E]"
            onClick={() => {
              mutation.mutate("default");
            }}
          >
            <ThemePreview
              shade1="fill-[#15181E]"
              shade2="fill-[#272B35]"
              shade3="fill-[#333947]"
            />
            <div class="flex gap-2 items-center w-full box-border justify-start px-2 py-1">
              <Show when={themeName() === "default"}>
                <div class="i-ri:check-fill text-shade-0" />
              </Show>
              <p class="m-0 text-shade-0">
                <Trans
                  key="default"
                  options={{
                    defaultValue: "default",
                  }}
                />
              </p>
            </div>
          </div>
          <div
            class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
            onClick={() => {
              mutation.mutate("light");
            }}
          >
            <ThemePreview
              shade1="fill-[#380505]"
              shade2="fill-[#A90F0F]"
              shade3="fill-[#E11313]"
            />
            <div class="flex justify-start items-center gap-2 w-full py-1 px-2 box-border">
              <Show when={themeName() === "light"}>
                <div class="i-ri:check-fill text-shade-0" />
              </Show>
              <p class="m-0 text-shade-0">
                <Trans
                  key="light"
                  options={{
                    defaultValue: "light",
                  }}
                />
              </p>
            </div>
          </div>
          <div
            class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
            onClick={() => {
              mutation.mutate("poison-green");
            }}
          >
            <ThemePreview
              shade1="fill-[#162009]"
              shade2="fill-[#43651B]"
              shade3="fill-[#598523]"
            />
            <div class="flex justify-start items-center gap-2 w-full py-1 px-2 box-border">
              <Show when={themeName() === "poison-green"}>
                <div class="i-ri:check-fill text-shade-0" />
              </Show>
              <p class="m-0 text-shade-0">
                <Trans
                  key="poison-green"
                  options={{
                    defaultValue: "poison-green",
                  }}
                />
              </p>
            </div>
          </div>
        </div>
      </div>
    </LoadingError>
  );
};

export default Appearance;
