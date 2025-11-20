import { Button } from "@gd/ui"
import { Show } from "solid-js"
import { Trans } from "@gd/i18n"

interface BulkActionsProps {
  selectedRowsLength: () => number
  isInstanceLocked: () => boolean
  onDeleteSelected: () => Promise<void>
  onUpdateSelected: () => Promise<void>
  hasUpdates: () => boolean
  onClearSelection: () => void
  class?: string
}

export const BulkActions = (props: BulkActionsProps) => {
  return (
    <Show when={props.selectedRowsLength() > 0}>
      <div
        class={`bg-darkSlate-700 border-darkSlate-600 mb-2 rounded-lg border p-2 sm:p-3 ${props.class || ""}`}
      >
        <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between md:gap-3">
          <span class="text-xs sm:text-sm">
            <Trans
              key="content:_trn_selected_count"
              options={{ count: props.selectedRowsLength() }}
            />
          </span>
          <div class="flex items-center gap-1 sm:gap-2 w-full md:w-auto">
            <Show when={props.hasUpdates()}>
              <Button
                size="small"
                type="secondary"
                disabled={props.isInstanceLocked()}
                onClick={props.onUpdateSelected}
                class="flex-1 md:flex-initial"
              >
                <div class="i-hugeicons:download-02" />
                <span class="hidden md:inline">
                  <Trans key="content:_trn_update_selected" />
                </span>
              </Button>
            </Show>
            <Button
              size="small"
              disabled={props.isInstanceLocked()}
              onClick={props.onDeleteSelected}
              class="flex-1 md:flex-initial"
            >
              <div class="i-hugeicons:delete-02" />
              <span class="hidden md:inline">
                <Trans key="content:_trn_delete_selected" />
              </span>
            </Button>
          </div>
        </div>
      </div>
    </Show>
  )
}
