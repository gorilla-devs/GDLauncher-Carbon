import type { FEWorld } from "@gd/core_module/bindings"
import { Button, PRESS_CLASSES, Skeleton } from "@gd/ui"
import { For, Match, Show, Switch } from "solid-js"
import { Trans } from "@gd/i18n"

export interface WorldsSidebarProps {
  worlds: FEWorld[]
  selected: string | undefined
  onSelect: (_name: string) => void
  isLoading: boolean
  isError: boolean
  onRetry: () => void
}

const WorldsSidebar = (props: WorldsSidebarProps) => {
  return (
    <div class="w-50 box-border flex h-full shrink-0 flex-col pr-6">
      <div class="box-border flex h-8 shrink-0 items-center gap-1.5 px-4 py-6">
        <span class="text-lightSlate-400 text-sm font-medium">
          <Trans key="content:_trn_tabs.worlds" />
        </span>
        <Show when={props.worlds.length > 0}>
          <span class="text-lightSlate-700 bg-darkSlate-700 rounded-full px-1.5 py-0.5 text-xs">
            {props.worlds.length}
          </span>
        </Show>
      </div>

      <Switch>
        <Match when={props.isError}>
          <div
            data-testid="worlds-load-failed"
            class="flex flex-col items-center gap-3 px-4 text-center"
          >
            <p class="text-lightSlate-600 m-0 text-sm">
              <Trans key="content:_trn_worlds.load_failed" />
            </p>
            <Button type="secondary" size="small" onClick={props.onRetry}>
              <Trans key="general:_trn_retry" />
            </Button>
          </div>
        </Match>
        <Match when={props.isLoading}>
          <Skeleton.logsList />
        </Match>
        <Match when={props.worlds.length === 0}>
          <p
            data-testid="worlds-none"
            class="text-lightSlate-600 m-0 px-4 text-sm"
          >
            <Trans key="content:_trn_worlds.none" />
          </p>
        </Match>
        <Match when={props.worlds.length > 0}>
          <div class="relative h-full overflow-y-auto">
            <For each={props.worlds}>
              {(world) => {
                const isSelected = () => props.selected === world.name
                const hasUpdate = () =>
                  world.datapacks.some((pack) => pack.has_update)
                return (
                  <div
                    data-testid="world-entry"
                    data-selected={isSelected()}
                    class={`group relative box-border flex w-full cursor-pointer items-center gap-2 rounded-md px-3 py-2 ${PRESS_CLASSES} ${isSelected() ? "bg-darkSlate-600" : "hover:bg-darkSlate-700"}`}
                    onClick={() => props.onSelect(world.name)}
                  >
                    <div class="flex min-w-0 flex-1 flex-col">
                      <span
                        class={`truncate text-sm ${isSelected() ? "text-lightSlate-100" : "text-lightSlate-500 group-hover:text-lightSlate-300"}`}
                      >
                        {world.name}
                      </span>
                      <Show when={world.identity !== null}>
                        <span class="text-lightSlate-700 text-xs">
                          <Show
                            when={world.datapacks.length > 0}
                            fallback={
                              <Trans key="content:_trn_worlds.no_datapacks" />
                            }
                          >
                            <Trans
                              key="content:_trn_worlds.datapack_count"
                              options={{ count: world.datapacks.length }}
                            />
                          </Show>
                        </span>
                      </Show>
                    </div>
                    <Show when={hasUpdate()}>
                      <div
                        data-testid="world-entry-update"
                        class="i-hugeicons:download-02 h-4 w-4 shrink-0 text-green-500"
                      />
                    </Show>
                  </div>
                )
              }}
            </For>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default WorldsSidebar
