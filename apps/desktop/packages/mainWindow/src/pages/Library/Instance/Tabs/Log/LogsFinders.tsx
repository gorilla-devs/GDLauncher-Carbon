import { Input, Popover, PopoverContent, PopoverTrigger } from "@gd/ui"
import { LogQuery } from "."
import { SetStoreFunction } from "solid-js/store"
import { Match, Setter, Switch } from "solid-js"
import { FESearchResult } from "@gd/core_module/bindings"
import { ScrollToIndexOpts } from "virtua/lib/core"
import { Trans, useTransContext } from "@gd/i18n"

interface Props {
  setSearchInputRef: (ref: HTMLInputElement) => void
  query: LogQuery
  setQuery: SetStoreFunction<LogQuery>
  logSearchResults: FESearchResult[] | undefined
  open: boolean
  setOpen: (open: boolean) => void
  currentResultIndex: number | null
  setCurrentResultIndex: Setter<number | null>
  scrollToIndex: (index: number, options: ScrollToIndexOpts) => void
  isIndexLoaded: (index: number) => boolean
}

const LogsFinders = (props: Props) => {
  const [t] = useTransContext()

  const handleArrowClick = async (direction: "up" | "down") => {
    if (!props.logSearchResults?.length) return

    let rowIndex: number
    if (props.currentResultIndex === null) {
      rowIndex = 0 // Start from first result
    } else {
      const newIndex =
        direction === "down"
          ? props.currentResultIndex + 1
          : props.currentResultIndex - 1
      rowIndex =
        (newIndex + (props.logSearchResults?.length ?? 0)) %
        (props.logSearchResults?.length ?? 0) // Cycle through
    }
    const isIndexLoaded = props.isIndexLoaded(
      props.logSearchResults?.[rowIndex]?.entry_index ?? 0
    )
    if (!isIndexLoaded) {
      props.scrollToIndex(
        props.logSearchResults?.[rowIndex]?.entry_index ?? 0,
        {}
      )
    }
    props.setCurrentResultIndex(rowIndex)

    requestAnimationFrame(() => {
      const currentResult = document.getElementById(
        `finder-result-${props.currentResultIndex}`
      )

      if (currentResult) {
        props.scrollToIndex(
          props.logSearchResults?.[rowIndex]?.entry_index ?? 0,
          {
            offset: currentResult.offsetTop - 100,
            smooth: true
          }
        )
      }
    })
  }

  return (
    <Popover open={props.open} gutter={4} placement="bottom">
      <PopoverTrigger
        as="div"
        onClick={() => props.setOpen(!props.open)}
        class="animate-icons-on-hover cursor-pointer"
        classList={{ "bg-darkSlate-700": props.open }}
      >
        <div
          class={`i-hugeicons:search-01 transition-all duration-200 ease-spring h-5 w-5 ${props.open ? "bg-lightSlate-50 rotate-12 scale-110" : "bg-lightSlate-800"}`}
        />
      </PopoverTrigger>
      <PopoverContent
        hideCloseButton
        class="bg-darkSlate-900 border-darkSlate-700 mr-40 w-fit border border-solid p-2"
      >
        <div class="flex items-center justify-between gap-4">
          <Input
            ref={(ref) => props.setSearchInputRef(ref)}
            icon={
              <div class="-mr-4 flex h-full items-center gap-0.5">
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.query.matchCase,
                    "hover:bg-darkSlate-800": !props.query.matchCase
                  }}
                  onClick={() =>
                    props.setQuery("matchCase", !props.query.matchCase)
                  }
                >
                  <div
                    class={`i-codicon:case-sensitive group-hover:bg-lightSlate-50 transition-colors duration-200 ease-spring ${props.query.matchCase ? "bg-lightSlate-50" : "bg-lightSlate-800"}`}
                  />
                </div>
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.query.matchWholeWord,
                    "hover:bg-darkSlate-800": !props.query.matchWholeWord
                  }}
                  onClick={() =>
                    props.setQuery(
                      "matchWholeWord",
                      !props.query.matchWholeWord
                    )
                  }
                >
                  <div
                    class={`i-codicon:whole-word group-hover:bg-lightSlate-50 transition-colors duration-200 ease-spring ${props.query.matchWholeWord ? "bg-lightSlate-50" : "bg-lightSlate-800"}`}
                  />
                </div>
                <div
                  class="group flex h-full items-center rounded-md px-2 transition-colors duration-200 ease-spring"
                  classList={{
                    "bg-darkSlate-500": props.query.useRegex,
                    "hover:bg-darkSlate-800": !props.query.useRegex
                  }}
                  onClick={() =>
                    props.setQuery("useRegex", !props.query.useRegex)
                  }
                >
                  <div
                    class={`i-codicon:regex group-hover:bg-lightSlate-50 transition-colors duration-200 ease-spring ${props.query.useRegex ? "bg-lightSlate-50" : "bg-lightSlate-800"}`}
                  />
                </div>
              </div>
            }
            class="h-6"
            placeholder={t("placeholders:_trn_find_logs")}
            value={props?.query?.query ?? ""}
            onInput={(e) => props.setQuery("query", e.target.value)}
          />
          <div class="text-lightSlate-800 text-sm">
            <Switch>
              <Match when={!props.logSearchResults?.length}>
                <div class="w-24">
                  <Trans key="ui:_trn_no_results" />
                </div>
              </Match>
              <Match when={props.logSearchResults?.length !== 0}>
                <div class="w-24">
                  {props.currentResultIndex !== null
                    ? `${props.currentResultIndex + 1} of ${props.logSearchResults?.length}`
                    : `1 of ${props.logSearchResults?.length}`}
                </div>
              </Match>
            </Switch>
          </div>
          <div
            class="i-hugeicons:arrow-up-01 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-spring h-4 w-4"
            onClick={() => handleArrowClick("up")}
          />
          <div
            class="i-hugeicons:arrow-down-01 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-spring h-4 w-4"
            onClick={() => handleArrowClick("down")}
          />
          <div
            class="i-hugeicons:cancel-01 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-spring h-4 w-4"
            onClick={() => props.setOpen(false)}
          />
        </div>
      </PopoverContent>
    </Popover>
  )
}

export default LogsFinders
