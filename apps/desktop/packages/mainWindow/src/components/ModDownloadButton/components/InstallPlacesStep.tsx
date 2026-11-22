import { For, JSX, Show } from "solid-js"
import { Trans } from "@gd/i18n"
import { DropdownMenuItem, DropdownMenuSeparator, Spinner } from "@gd/ui"
import type { InstallPlaces, ReadableWorld } from "@/utils/worldTarget"
import { worldNames } from "@/utils/worldLabel"

interface InstallPlacesStepProps {
  /** Undefined while the instance's worlds and addons are read. */
  places: InstallPlaces | undefined
  /** Shown above the choices: the picker's way back, naming the instance. */
  header?: JSX.Element
  onWorld: (world: ReadableWorld) => void
  onInstance: () => void
}

/** The "where" step of a data pack install, inside a dropdown menu: the
 *  instance's worlds as a group, then the instance itself. A choice keeps the
 *  menu open; the caller decides what it shows next. */
export const InstallPlacesStep = (props: InstallPlacesStepProps) => (
  <div class="p-1">
    {props.header}
    <Show
      when={props.places}
      fallback={
        <div class="flex justify-center py-3">
          <Spinner class="h-4 w-4" />
        </div>
      }
    >
      {(places) => {
        const names = () => worldNames(places().worlds.map((e) => e.world))
        return (
          <>
            <Show when={places().worlds.length > 0}>
              <div
                data-group-name
                class="text-lightSlate-500 px-2 pb-0.5 pt-1.5 text-xs"
              >
                <Trans key="content:_trn_tabs.worlds" />
              </div>
            </Show>
            <div class="max-h-[300px] overflow-y-auto">
              <For each={places().worlds}>
                {(entry) => (
                  <DropdownMenuItem
                    class="gap-2"
                    disabled={entry.holds}
                    closeOnSelect={false}
                    onSelect={() => props.onWorld(entry.world)}
                  >
                    <div class="i-hugeicons:globe h-4 w-4 shrink-0" />
                    <span class="min-w-0 flex-1 truncate">
                      {names().get(entry.world.name)?.label}
                      <Show when={names().get(entry.world.name)?.folder}>
                        {(folder) => (
                          <span class="text-lightSlate-600"> {folder()}</span>
                        )}
                      </Show>
                    </span>
                    <Show when={entry.holds}>
                      <div class="i-hugeicons:tick-02 h-4 w-4 shrink-0 text-green-400" />
                    </Show>
                  </DropdownMenuItem>
                )}
              </For>
            </div>
            <Show when={places().worlds.length > 0}>
              <DropdownMenuSeparator class="-mx-1" />
            </Show>
            <DropdownMenuItem
              class="gap-2"
              disabled={places().instanceHolds}
              closeOnSelect={false}
              onSelect={() => props.onInstance()}
            >
              <div class="i-hugeicons:folder-01 h-4 w-4 shrink-0" />
              <span class="min-w-0 flex-1 truncate">
                <Trans key="content:_trn_worlds.download_to_instance" />
              </span>
              <Show when={places().instanceHolds}>
                <div class="i-hugeicons:tick-02 h-4 w-4 shrink-0 text-green-400" />
              </Show>
            </DropdownMenuItem>
          </>
        )
      }}
    </Show>
  </div>
)
