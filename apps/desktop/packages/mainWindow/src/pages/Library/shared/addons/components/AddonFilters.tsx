import {
  Badge,
  Button,
  Input,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { For, Show, onMount, onCleanup, createEffect } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getAddonTabKey } from "@gd/i18n/helpers"
import { getAddonTypeIcon } from "@/utils/addonIcons"
import { AddonFiltersProps } from "../types"

export const AddonFilters = (props: AddonFiltersProps) => {
  const [t] = useTransContext()
  let containerRef: HTMLDivElement | undefined

  const measureHeight = () => {
    if (containerRef && props.onHeightChange) {
      props.onHeightChange(containerRef.offsetHeight)
    }
  }

  onMount(() => {
    if (containerRef && props.onHeightChange) {
      measureHeight()

      const resizeObserver = new ResizeObserver(() => {
        measureHeight()
      })

      resizeObserver.observe(containerRef)

      onCleanup(() => {
        resizeObserver.disconnect()
      })
    }
  })

  createEffect(() => {
    props.addonTypes() // Track dependency
    const timeoutId = setTimeout(measureHeight, 0)
    onCleanup(() => clearTimeout(timeoutId))
  })

  return (
    <div
      ref={containerRef}
      class={`bg-darkSlate-800 border-darkSlate-700 sticky z-30 border-b px-6 pb-6 pt-0 ${props.stickyTop ?? "top-0"}`}
    >
      <div class="flex flex-col gap-4">
        {/* Search and main actions */}
        <div class="flex items-center gap-2">
          <Input
            value={props.searchQuery()}
            onInput={(e) => props.setSearchQuery(e.target.value)}
            placeholder={t("content:_trn_search_addons")}
            icon={<div class="i-hugeicons:search-01" />}
            class={props.searchInputClass ?? "min-w-0 flex-1"}
          />

          {/* Extra actions slot (platform filter, update-all, etc.) */}
          {props.extraActions}

          <Tooltip>
            <TooltipTrigger>
              <Button
                type="secondary"
                size="small"
                onClick={props.onOpenFolder}
                class="px-2"
              >
                <div class="i-hugeicons:folder-open" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="instances:_trn_open_folder" />
            </TooltipContent>
          </Tooltip>

          <Show
            when={!props.addButtonTooltip}
            fallback={
              <Tooltip open={props.addButtonDisabled ? undefined : false}>
                <TooltipTrigger>
                  <Button
                    type="primary"
                    data-testid="addons-add-button"
                    size="small"
                    onClick={props.onAddAddons}
                    disabled={props.addButtonDisabled}
                    class="font-semibold"
                  >
                    <div class="i-hugeicons:add-01" />
                    <span class="hidden md:inline">
                      <Trans key="content:_trn_add_addons" />
                    </span>
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{props.addButtonTooltip}</TooltipContent>
              </Tooltip>
            }
          >
            <Button
              type="primary"
              data-testid="addons-add-button"
              size="small"
              onClick={props.onAddAddons}
              disabled={props.addButtonDisabled}
              class="font-semibold"
            >
              <div class="i-hugeicons:add-01" />
              <span class="hidden md:inline">
                <Trans key="content:_trn_add_addons" />
              </span>
            </Button>
          </Show>
        </div>

        {/* Addon type filters */}
        <div class="mt-2 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div class="flex flex-wrap items-center gap-2">
            <For each={props.addonTypes()}>
              {(type) => (
                <Badge
                  variant={
                    props.enabledAddonTypes[type] ? "default" : "secondary"
                  }
                  class="flex cursor-pointer items-center gap-1.5"
                  onClick={() => {
                    props.setEnabledAddonTypes(
                      type,
                      !props.enabledAddonTypes[type]
                    )
                  }}
                >
                  <div class={`${getAddonTypeIcon(type as any)} text-sm`} />
                  <span class="hidden md:inline">
                    {t(getAddonTabKey(type as any))}
                  </span>
                  <Show when={props.enabledAddonTypes[type]}>
                    <div class="i-hugeicons:tick-02 ml-1" />
                  </Show>
                </Badge>
              )}
            </For>
          </div>
          <div class="text-lightSlate-600 hidden shrink-0 items-center gap-2 text-xs xl:flex">
            <div class="i-hugeicons:mouse-01" />
            <span>{t("content:_trn_right_click_hint")}</span>
            <span class="text-lightSlate-700">&bull;</span>
            <span>{t("content:_trn_multi_select_hint")}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
