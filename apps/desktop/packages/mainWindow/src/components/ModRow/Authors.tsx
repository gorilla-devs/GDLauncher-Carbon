import { ModRowProps } from "@/utils/mods"
import { Accessor, For, Show } from "solid-js"
import { Tooltip, TooltipTrigger, TooltipContent } from "@gd/ui"
import {
  FEUnifiedSearchResult,
  FEUnifiedAuthor
} from "@gd/core_module/bindings"

interface Props {
  modProps: ModRowProps | FEUnifiedSearchResult
  isRowSmall?: Accessor<boolean>
}

export const getAuthors = (
  prop: ModRowProps | FEUnifiedSearchResult
): FEUnifiedAuthor[] => {
  const isModRow = "data" in prop
  if (isModRow) {
    return prop.data.authors
  } else {
    return prop.authors
  }
}

const Authors = (props: Props) => {
  const authors = () => getAuthors(props.modProps)

  return (
    <div class="text-sm flex gap-2 whitespace-nowrap">
      <Show
        when={!props?.isRowSmall?.()}
        fallback={
          <>
            <p class="m-0">{authors()[0]?.name}</p>
            <Show when={authors().length - 1 > 0}>
              <Tooltip>
                <TooltipTrigger>
                  <p class="m-0">{`+${authors().length - 1}`}</p>
                </TooltipTrigger>
                <TooltipContent>
                  <div class="flex gap-2">
                    <For each={authors().slice(1)}>
                      {(author) => <p class="m-0">{author?.name}</p>}
                    </For>
                  </div>
                </TooltipContent>
              </Tooltip>
            </Show>
          </>
        }
      >
        <For each={authors().slice(0, 2)}>
          {(author, i) => (
            <>
              <p class="m-0">{author?.name}</p>
              <Show when={i() !== authors().slice(0, 2).length - 1}>
                <span class="text-lightSlate-100">{"•"}</span>
              </Show>
            </>
          )}
        </For>
        <Show when={authors().length > 2}>
          <Tooltip>
            <TooltipTrigger>
              <p class="m-0">{`+${authors().slice(2).length}`}</p>
            </TooltipTrigger>
            <TooltipContent>
              <div class="flex gap-2">
                <For each={authors().slice(2)}>
                  {(author) => <p class="m-0">{author?.name}</p>}
                </For>
              </div>
            </TooltipContent>
          </Tooltip>
        </Show>
      </Show>
    </div>
  )
}

export default Authors
