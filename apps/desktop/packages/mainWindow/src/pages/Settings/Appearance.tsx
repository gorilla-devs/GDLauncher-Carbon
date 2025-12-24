import ThemePreview from "@/components/ThemePreview"
import { queryClient, rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { getThemeKey } from "@gd/i18n/helpers"
import { useRouteData } from "@solidjs/router"
import { For } from "solid-js"
import fetchData from "./settings.general.data"
import LoadingError from "@/components/LoadingError"
import {
  applyThemeByName,
  getAvailableThemes,
  getThemeColor
} from "@/utils/theme"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"

const Appearance = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const themeName = () => routeData?.data?.data?.theme || "main"

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"],
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme)
    }
  }))

  const themes = getAvailableThemes()

  return (
    <LoadingError routeData={routeData}>
      <div class="w-full">
        <PageTitle>
          <Trans key="settings:_trn_appearance" />
        </PageTitle>
        <Row forceContentBelow class="border-box w-full max-w-full">
          <Title
            description={<Trans key="settings:_trn_default_themes_text" />}
          >
            <Trans key="settings:_trn_default_themes_title" />
          </Title>
          <div class="flex w-full flex-wrap gap-6">
            <For each={themes}>
              {(theme) => {
                const shade1 = getThemeColor(theme, "darkSlate-900")
                const shade2 = getThemeColor(theme, "darkSlate-700")
                const shade3 = getThemeColor(theme, "darkSlate-600")

                return (
                  <div>
                    <div
                      class="w-42 relative rounded-md p-2 transition-transform duration-300 ease-spring hover:scale-105"
                      style={{ "background-color": shade1 }}
                      classList={{
                        "scale-105": themeName() === theme
                      }}
                      onClick={() => {
                        settingsMutation.mutate({
                          theme: {
                            Set: theme
                          }
                        })
                      }}
                      onMouseEnter={() => {
                        applyThemeByName(theme)
                      }}
                      onMouseLeave={() => {
                        applyThemeByName(themeName())
                      }}
                    >
                      <div
                        class="bg-darkSlate-500 absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full p-2 opacity-0 shadow-md transition-opacity duration-300 ease-spring"
                        classList={{
                          "opacity-100": themeName() === theme
                        }}
                      >
                        <div class="i-hugeicons:tick-02 h-6 w-6" />
                      </div>

                      <ThemePreview
                        shade1={shade1}
                        shade2={shade2}
                        shade3={shade3}
                      />
                    </div>
                    <div class="box-border flex w-full items-center justify-start gap-2 px-2 py-4">
                      <p class="text-lightSlate-700 m-0">
                        <Trans key={getThemeKey(theme)} />
                      </p>
                    </div>
                  </div>
                )
              }}
            </For>
          </div>
        </Row>
        <Row forceContentBelow class="border-box w-full max-w-full">
          <Title description={<Trans key="settings:_trn_custom_themes_text" />}>
            <Trans key="settings:_trn_custom_themes_title" />
          </Title>
          <div class="flex w-full flex-wrap gap-6">
            <Trans key="general:_trn_coming_soon" />
          </div>
        </Row>
      </div>
    </LoadingError>
  )
}

export default Appearance
