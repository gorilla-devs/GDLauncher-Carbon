import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For, Show } from "solid-js";
import fetchData from "./settings.general.data";
import LoadingError from "@/components/LoadingError";
import { getAvailableThemes, getThemeColors } from "@/utils/theme";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const themeName = () => routeData?.data?.data?.theme || "default";

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme);
    },
  });

  // const anotherTheme = getThemeColors(anotherThemeName);
  const themes = getAvailableThemes();

  return (
    <LoadingError routeData={routeData}>
      <>
        <PageTitle>
          <Trans key="settings:Appearance" />
        </PageTitle>
        <Row class="gap-4 w-full border-box flex-wrap max-w-full">
          <For each={themes}>
            {(theme) => {
              const themeColors = getThemeColors(theme);

              const shade1 = themeColors && themeColors["darkSlate-900"];
              const shade2 = themeColors && themeColors["darkSlate-700"];
              const shade3 = themeColors && themeColors["darkSlate-600"];

              return (
                <div
                  class="flex flex-col justify-center items-center cursor-pointer inline-flex p-1 w-42"
                  style={{ "background-color": shade1 }}
                  onClick={() => {
                    settingsMutation.mutate({
                      theme: theme,
                    });
                  }}
                >
                  <ThemePreview
                    shade1={shade1}
                    shade2={shade2}
                    shade3={shade3}
                  />
                  <div class="flex gap-2 items-center w-full box-border px-2 py-1 justify-start">
                    <Show when={themeName() === theme}>
                      <div class="text-darkSlate-50 i-ri:check-fill" />
                    </Show>
                    <p class="m-0 text-darkSlate-50">
                      <Trans
                        key={`settings.theme_${theme}`}
                        options={{
                          defaultValue: "default",
                        }}
                      />
                    </p>
                  </div>
                </div>
              );
            }}
          </For>
        </Row>
      </>
    </LoadingError>
  );
};

export default Appearance;
