import { ModRowProps } from "@/utils/mods"
import { For, Show } from "solid-js"
import { Tag, Tooltip, TooltipTrigger, TooltipContent } from "@gd/ui"
import { capitalize } from "@/utils/helpers"

interface Props {
  modProps: ModRowProps
  isRowSmall: boolean
}

const Categories = (props: Props) => {
  const categories = () => props.modProps.data.categories

  return (
    <div class="flex gap-2 scrollbar-hide">
      <Show
        when={!props.isRowSmall}
        fallback={
          <>
            <Show when={categories().length > 0}>
              <Tooltip>
                <TooltipTrigger>
                  <Tag
                    name={capitalize(String(categories()[0])) ?? undefined}
                    type="fixed"
                  />
                </TooltipTrigger>
                <TooltipContent>
                  {capitalize(String(categories()[0]))}
                </TooltipContent>
              </Tooltip>
            </Show>
            <Show when={categories().length - 1 > 0}>
              <Tooltip>
                <TooltipTrigger>
                  <Tag name={`+${categories().length - 1}`} type="fixed" />
                </TooltipTrigger>
                <TooltipContent>
                  <div class="flex gap-2">
                    <For each={categories().slice(1)}>
                      {(category) => (
                        <Tag
                          name={capitalize(String(category)) ?? undefined}
                          type="fixed"
                        />
                      )}
                    </For>
                  </div>
                </TooltipContent>
              </Tooltip>
            </Show>
          </>
        }
      >
        <For each={categories()}>
          {(category) => (
            <Tooltip>
              <TooltipTrigger>
                <Tag
                  name={capitalize(String(category)) ?? undefined}
                  type="fixed"
                />
              </TooltipTrigger>
              <TooltipContent>{capitalize(String(category))}</TooltipContent>
            </Tooltip>
          )}
        </For>
      </Show>
    </div>
  )
}

export default Categories
