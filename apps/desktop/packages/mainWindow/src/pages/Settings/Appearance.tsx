import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For } from "solid-js";
import fetchData from "./settings.general.data";
import LoadingError from "@/components/LoadingError";
import { getAvailableThemes, getThemeColor } from "@/utils/theme";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const themeName = () => routeData?.data?.data?.theme || "default";

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"],
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme);
    }
  }));

  const themes = getAvailableThemes();

  return (
    <LoadingError routeData={routeData}>
      <div class="w-full">
        <PageTitle>
          <Trans key="settings:Appearance" />
        </PageTitle>
        <Row forceContentBelow class="w-full border-box max-w-full">
          <Title description={<Trans key="settings:default_themes_text" />}>
            <Trans key="settings:default_themes_title" />
          </Title>
          <div class="w-full flex flex-wrap gap-6">
            <For each={themes}>
              {(theme) => {
                const shade1 = getThemeColor(theme, "darkSlate-900");
                const shade2 = getThemeColor(theme, "darkSlate-700");
                const shade3 = getThemeColor(theme, "darkSlate-600");

                return (
                  <div>
                    <div
                      class="relative p-2 w-42 rounded-md hover:scale-105 transition-transform duration-300 ease-in-out"
                      style={{ "background-color": shade1 }}
                      classList={{
                        "scale-105": themeName() === theme
                      }}
                      onClick={() => {
                        settingsMutation.mutate({
                          theme: {
                            Set: theme
                          }
                        });
                      }}
                    >
                      <div
                        class="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-darkSlate-500 shadow-md rounded-full p-2 opacity-0 transition-opacity duration-300 ease-in-out"
                        classList={{
                          "opacity-100": themeName() === theme
                        }}
                      >
                        <div class="i-ri:check-fill w-6 h-6" />
                      </div>

                      <ThemePreview
                        shade1={shade1}
                        shade2={shade2}
                        shade3={shade3}
                      />
                    </div>
                    <div class="flex gap-2 items-center w-full box-border px-2 py-4 justify-start">
                      <p class="m-0 text-darkSlate-50">
                        <Trans key={`settings:theme_${theme}`} />
                      </p>
                    </div>
                  </div>
                );
              }}
            </For>
          </div>
        </Row>
        <Row forceContentBelow class="w-full border-box max-w-full">
          <Title description={<Trans key="settings:custom_themes_text" />}>
            <Trans key="settings:custom_themes_title" />
          </Title>
          <div class="w-full flex flex-wrap gap-6">
            <Trans key="general.coming_soon" />
          </div>
        </Row>
      </div>
    </LoadingError>
  );
};

export default Appearance;
