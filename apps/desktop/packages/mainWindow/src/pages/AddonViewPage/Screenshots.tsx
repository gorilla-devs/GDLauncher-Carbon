import { Trans } from "@gd/i18n"
import { For, Match, Suspense, Switch, useContext } from "solid-js"
import { Skeleton } from "@gd/ui"
import { AddonContext } from "."
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

const Screenshots = () => {
  const mod = useContext(AddonContext)

  const screenshots = () => {
    return mod?.data?.screenshotUrls
  }

  return (
    <Suspense fallback={<Skeleton.modpackScreenshotsPage />}>
      <div>
        <Switch fallback={<Skeleton.modpackScreenshotsPage />}>
          <Match when={(screenshots()?.length || 0) > 0 && !mod?.isLoading}>
            <div class="flex flex-col gap-4">
              <div class="flex flex-wrap gap-4">
                <For each={screenshots()}>
                  {(screenshot) => (
                    <img
                      src={screenshot}
                      class="h-44 w-72 rounded-xl"
                      alt={screenshot}
                    />
                  )}
                </For>
              </div>
            </div>
          </Match>
          <Match when={(screenshots()?.length || 0) === 0 && !mod?.isLoading}>
            <div class="flex flex-col items-center justify-center gap-6 py-12 text-center">
              <PlaceholderGorilla
                size={8}
                variant="Gallery Gorilla - Empty Frame"
              />
              <p class="text-lightSlate-700 max-w-100">
                <Trans
                  key="content:_trn_modpack.no_screenshot"
                  options={{
                    defaultValue: "No screenshots"
                  }}
                />
              </p>
            </div>
          </Match>
          <Match when={mod?.isLoading}>
            <Skeleton.modpackScreenshotsPage />
          </Match>
        </Switch>
      </div>
    </Suspense>
  )
}

export default Screenshots
