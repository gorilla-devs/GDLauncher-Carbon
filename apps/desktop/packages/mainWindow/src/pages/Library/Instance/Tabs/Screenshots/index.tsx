import { Trans } from "@gd/i18n"
import { Checkbox } from "@gd/ui"
import { For, Show, createEffect } from "solid-js"
import { createStore } from "solid-js/store"
import { format } from "date-fns"
import { getTitleByDays } from "@/utils/helpers"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

interface ScreenshotsType {
  img: string
  date: string
}

const screenshots: ScreenshotsType[] = [
  // {
  //   img: screenshot1,
  //   date: "2023-01-31T09:20:53.513Z"
  // },
  // {
  //   img: screenshot1,
  //   date: "2023-01-31T09:20:53.513Z"
  // },
  // {
  //   img: screenshot1,
  //   date: "2023-01-28T09:20:53.513Z"
  // },
  // {
  //   img: screenshot1,
  //   date: "2023-01-29T09:20:53.513Z"
  // },
  // {
  //   img: screenshot1,
  //   date: "2023-01-30T09:20:53.513Z"
  // },
  // {
  //   img: screenshot1,
  //   date: "2023-01-29T09:20:53.513Z"
  // }
]

const NoMods = () => {
  return (
    <div class="min-h-90 flex h-full w-full items-center justify-center">
      <div class="flex flex-col items-center justify-center gap-6 text-center">
        <PlaceholderGorilla size={10} variant="Camera Gorilla - Photographer" />
        <p class="text-lightSlate-700 max-w-100">
          <Trans
            key="instance.no_screenshots_text"
            options={{
              defaultValue:
                "You don't have any screenshots related to this modpack at the moment, to take a screenshot press the print screen key during the game"
            }}
          />
        </p>
      </div>
    </div>
  )
}

interface MappedScreenshots extends ScreenshotsType {
  timestamp: Date
  days: number
}

type FilteredScreenshots = Record<string, MappedScreenshots[]>

const Screenshots = () => {
  const [filteredScreenshots, setFilteredScreenshots] =
    createStore<FilteredScreenshots>({})

  createEffect(() => {
    const filteredscreenshots: MappedScreenshots[] = []
    screenshots.map((screenshot) => {
      const fileBirthdate = new Date(screenshot.date)
      const timeDiff: number = Date.now() - (fileBirthdate as any)
      const days = Math.floor(timeDiff / 1000) / 60 / 60 / 24
      filteredscreenshots.push({
        ...screenshot,
        timestamp: fileBirthdate,
        days
      })
    })
    const sortedScreenshots = filteredscreenshots.sort(
      (a, b) => (b.timestamp as any) - (a.timestamp as any)
    )

    const hashmapDates = new Map()

    for (const screenshot of sortedScreenshots) {
      if (hashmapDates.has(screenshot.days)) {
        hashmapDates.set(screenshot.days, [
          ...hashmapDates.get(screenshot.days),
          screenshot
        ])
      } else {
        hashmapDates.set(screenshot.days, [screenshot])
      }
    }

    setFilteredScreenshots(Object.fromEntries(hashmapDates))
  })

  return (
    <div>
      <div class="top-30 bg-darkSlate-800 sticky z-10 flex flex-col pt-10 transition-all duration-100 ease-in-out">
        <div class="text-lightSlate-700 z-10 mb-5 flex justify-between">
          <div class="flex gap-4">
            <div class="flex cursor-pointer items-center gap-2">
              <Checkbox checked={true} disabled={false} />
              <Trans
                key="instance.select_all_screenshots"
                options={{
                  defaultValue: "Select All"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-in-out">
              <div class="i-hugeicons:folder-open text-2xl" />
              <Trans
                key="instance.open_screenshots_folder"
                options={{
                  defaultValue: "Open folder"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-in-out">
              <div class="i-hugeicons:unavailable text-2xl" />
              <Trans
                key="instance.disable_screenshot"
                options={{
                  defaultValue: "disable"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-in-out">
              <div class="i-hugeicons:delete-02 text-2xl" />
              <Trans
                key="instance.delete_screenshot"
                options={{
                  defaultValue: "delete"
                }}
              />
            </div>
          </div>
          <div class="flex gap-2">
            <p class="m-0">{screenshots.length}</p>
            <Trans
              key="instance.screenshots"
              options={{
                defaultValue: "Screenshots"
              }}
            />
          </div>
        </div>
      </div>
      <div class="flex h-full flex-col gap-10 overflow-y-hidden">
        <Show when={screenshots.length > 0} fallback={<NoMods />}>
          <For each={Object.entries(filteredScreenshots)}>
            {([days, screenshots]) => (
              <div class="flex flex-col">
                <h3 class="mt-0">{getTitleByDays(days)}</h3>
                <div class="flex h-auto w-full flex-wrap gap-6">
                  <For each={screenshots}>
                    {(screenshot) => (
                      <div class="flex flex-col">
                        <img class="h-32 w-60" src={screenshot.img} />
                        <div class="mt-2 flex items-center justify-between">
                          <p class="text-lightSlate-700 text-md m-0">
                            {format(new Date(screenshot.date), "dd-MM-yyyy")}
                          </p>
                          <div class="i-hugeicons:more-horizontal text-lightSlate-700" />
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  )
}

export default Screenshots
