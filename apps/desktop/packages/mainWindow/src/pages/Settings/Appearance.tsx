import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For, Show } from "solid-js";
import fetchData from "./settings.general.data";
import LoadingError from "@/components/LoadingError";
import { getAvailableThemes, getThemeColor } from "@/utils/theme";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const themeName = () => routeData?.data?.data?.theme || "default";

  function addTemporaryStyle() {
    const element_id = "gdl-theme-temp-transition";
    const element = document.getElementById(element_id);

    if (element) {
      return;
    }

    // Create a <style> element
    const style = document.createElement("style");
    style.id = "gdl-theme-temp-transition";

    // Add CSS rules to the <style> element
    style.innerHTML = `* {
      transition: all 0.4s ease-in-out;
    }`;

    document.head.appendChild(style);
    setTimeout(() => {
      document.head.removeChild(style);
    }, 600);
  }

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"],
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme);
    }
  }));

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
              const shade1 = getThemeColor(theme, "darkSlate-900");
              const shade2 = getThemeColor(theme, "darkSlate-700");
              const shade3 = getThemeColor(theme, "darkSlate-600");

              return (
                <div>
                  <div
                    class="flex flex-col justify-center items-center cursor-pointer inline-flex p-2 w-42 rounded-md"
                    style={{ "background-color": shade1 }}
                    onClick={() => {
                      window.document.body.style;

                      settingsMutation.mutate({
                        theme: {
                          Set: theme
                        }
                      });
                    }}
                  >
                    <ThemePreview
                      shade1={shade1}
                      shade2={shade2}
                      shade3={shade3}
                    />
                  </div>
                  <div class="flex gap-2 items-center w-full box-border px-2 py-1 justify-start">
                    <Show when={themeName() === theme}>
                      <div class="text-darkSlate-50 i-ri:check-fill" />
                    </Show>
                    <p class="m-0 text-darkSlate-50">
                      <Trans key={`settings:theme_${theme}`} />
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
