import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { Trans } from "@gd/i18n"
import { For, Show } from "solid-js"
import {
  BlockedText,
  type ChangesBlocked
} from "@/pages/Library/shared/addons/components/ChangesBlocked"
import type { ReadableWorld } from "../hooks/useDatapackWorlds"

const ICON = "i-hugeicons:add-circle h-5 w-5 transition-colors"

/** The "Add to a world" icon of a data pack row and its menu of worlds. */
export const AddToWorldMenu = (props: {
  worlds: readonly ReadableWorld[]
  loading: boolean
  failed: boolean
  blocked: ChangesBlocked
  pending: boolean
  onChoose: (_world: ReadableWorld) => void
}) => (
  <Show
    when={!props.blocked && !props.pending}
    fallback={
      <Tooltip>
        <TooltipTrigger>
          <div
            data-testid="addon-add-to-world"
            data-disabled="true"
            class={`${ICON} text-lightSlate-700 cursor-not-allowed`}
          />
        </TooltipTrigger>
        <TooltipContent>
          <Show
            when={props.blocked}
            fallback={<Trans key="content:_trn_worlds.add_to_world" />}
          >
            <BlockedText blocked={props.blocked} />
          </Show>
        </TooltipContent>
      </Tooltip>
    }
  >
    <DropdownMenu placement="bottom-end">
      <Tooltip>
        <TooltipTrigger as="div">
          <DropdownMenuTrigger class="b-0 bg-transparent p-0">
            <div
              data-testid="addon-add-to-world"
              class={`${ICON} text-lightSlate-400 hover:text-lightSlate-200 cursor-pointer`}
            />
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent>
          <Trans key="content:_trn_worlds.add_to_world" />
        </TooltipContent>
      </Tooltip>
      <DropdownMenuContent>
        <Show
          when={props.worlds.length > 0}
          fallback={
            <div
              data-testid="addon-add-to-world-empty"
              class="text-lightSlate-600 max-w-64 px-3 py-2 text-sm"
            >
              <Show
                when={!props.loading}
                fallback={<Spinner class="h-4 w-4" />}
              >
                <Show
                  when={props.failed}
                  fallback={<Trans key="content:_trn_worlds.none" />}
                >
                  <Trans key="content:_trn_worlds.load_failed" />
                </Show>
              </Show>
            </div>
          }
        >
          <For each={props.worlds}>
            {(world) => (
              <DropdownMenuItem
                data-testid="addon-add-to-world-item"
                onSelect={() => props.onChoose(world)}
              >
                {world.name}
              </DropdownMenuItem>
            )}
          </For>
        </Show>
      </DropdownMenuContent>
    </DropdownMenu>
  </Show>
)
