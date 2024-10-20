import { Trans } from "@gd/i18n";
import { Checkbox } from "@gd/ui";
import { For, Show, createEffect } from "solid-js";
import pictureImage from "/assets/images/icons/picture.png";
import { createStore } from "solid-js/store";
import { format } from "date-fns";
import { getTitleByDays } from "@/utils/helpers";

type ScreenshotsType = {
  img: string;
  date: string;
};

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
];

const NoMods = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={pictureImage} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
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
  );
};

interface MappedScreenshots extends ScreenshotsType {
  timestamp: Date;
  days: number;
}

interface FilteredScreenshots {
  [timestamp: string]: MappedScreenshots[];
}

const Screenshots = () => {
  const [filteredScreenshots, setFilteredScreenshots] =
    createStore<FilteredScreenshots>({});

  createEffect(() => {
    const filteredscreenshots: MappedScreenshots[] = [];
    screenshots.map((screenshot) => {
      const fileBirthdate = new Date(screenshot.date);
      const timeDiff: number = Date.now() - (fileBirthdate as any);
      const days = Math.floor(timeDiff / 1000) / 60 / 60 / 24;
      filteredscreenshots.push({
        ...screenshot,
        timestamp: fileBirthdate,
        days
      });
    });
    const sortedScreenshots = filteredscreenshots.sort(
      (a, b) => (b.timestamp as any) - (a.timestamp as any)
    );

    const hashmapDates = new Map();

    for (const screenshot of sortedScreenshots) {
      if (hashmapDates.has(screenshot.days)) {
        hashmapDates.set(screenshot.days, [
          ...hashmapDates.get(screenshot.days),
          screenshot
        ]);
      } else {
        hashmapDates.set(screenshot.days, [screenshot]);
      }
    }

    setFilteredScreenshots(Object.fromEntries(hashmapDates));
  });

  return (
    <div>
      <div class="flex flex-col sticky top-30 bg-darkSlate-800 z-10 pt-10 transition-all duration-100 ease-in-out">
        <div class="flex justify-between text-darkSlate-50 z-10 mb-5">
          <div class="flex gap-4">
            <div class="flex items-center gap-2 cursor-pointer">
              <Checkbox checked={true} disabled={false} />
              <Trans
                key="instance.select_all_screenshots"
                options={{
                  defaultValue: "Select All"
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-lightSlate-50 transition duration-100 ease-in-out">
              <span class="i-ri:folder-open-fill text-2xl" />
              <Trans
                key="instance.open_screenshots_folder"
                options={{
                  defaultValue: "Open folder"
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-lightSlate-50 transition duration-100 ease-in-out">
              <span class="i-ri:forbid-line text-2xl" />
              <Trans
                key="instance.disable_screenshot"
                options={{
                  defaultValue: "disable"
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-lightSlate-50 transition duration-100 ease-in-out">
              <span class="i-ri:delete-bin-2-fill text-2xl" />
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
      <div class="h-full overflow-y-hidden flex flex-col gap-10">
        <Show when={screenshots.length > 0} fallback={<NoMods />}>
          <For each={Object.entries(filteredScreenshots)}>
            {([days, screenshots]) => (
              <div class="flex flex-col">
                <h3 class="mt-0">{getTitleByDays(days)}</h3>
                <div class="w-full flex gap-6 flex-wrap h-auto">
                  <For each={screenshots}>
                    {(screenshot) => (
                      <div class="flex flex-col">
                        <img class="w-60 h-32" src={screenshot.img} />
                        <div class="flex justify-between items-center mt-2">
                          <p class="m-0 text-darkSlate-50 text-md">
                            {format(new Date(screenshot.date), "dd-MM-yyyy")}
                          </p>
                          <div class="text-darkSlate-50 i-ri:more-2-fill" />
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
  );
};

export default Screenshots;
