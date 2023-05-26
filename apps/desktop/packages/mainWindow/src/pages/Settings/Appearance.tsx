import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { Show } from "solid-js";
import fetchData from "./settings.general.data";
import LoadingError from "@/components/LoadingError";

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const themeName = () => routeData?.data?.data?.theme || "default";

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], {
        theme: newTheme.theme,
      });
    },
  });

  return (
    <LoadingError routeData={routeData}>
      <div class="bg-darkSlate-800 w-full h-auto flex flex-col py-5 px-6 box-border">
        <h2 class="m-0 mb-7 text-4">
          <Trans
            key="settings.appearance"
            options={{
              defaultValue: "Appearance",
            }}
          />
        </h2>
        <div class="flex justify-between w-full border-box max-w-[35rem]">
          <div
            class="flex flex-col flex justify-center items-center cursor-pointer w-42 p-1 bg-[#15181E]"
            onClick={() => {
              settingsMutation.mutate({
                theme: "default",
              });
            }}
          >
            <ThemePreview
              shade1="fill-[#15181E]"
              shade2="fill-[#272B35]"
              shade3="fill-[#333947]"
            />
            <div class="flex gap-2 items-center w-full box-border justify-start px-2 py-1">
              <Show when={themeName() === "default"}>
                <div class="i-ri:check-fill text-darkSlate-50" />
              </Show>
              <p class="m-0 text-darkSlate-50">
                <Trans
                  key="settings.theme_default"
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
              settingsMutation.mutate({
                theme: "light",
              });
            }}
          >
            <ThemePreview
              shade1="fill-[#380505]"
              shade2="fill-[#A90F0F]"
              shade3="fill-[#E11313]"
            />
            <div class="flex justify-start items-center gap-2 w-full py-1 px-2 box-border">
              <Show when={themeName() === "light"}>
                <div class="i-ri:check-fill text-darkSlate-50" />
              </Show>
              <p class="m-0 text-darkSlate-50">
                <Trans
                  key="settings.theme_light"
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
              settingsMutation.mutate({
                theme: "poison-green",
              });
            }}
          >
            <ThemePreview
              shade1="fill-[#162009]"
              shade2="fill-[#43651B]"
              shade3="fill-[#598523]"
            />
            <div class="flex justify-start items-center gap-2 w-full py-1 px-2 box-border">
              <Show when={themeName() === "poison-green"}>
                <div class="i-ri:check-fill text-darkSlate-50" />
              </Show>
              <p class="m-0 text-darkSlate-50">
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
