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
        class={`bg-darkSlate-700 border-darkSlate-600 mb-2 rounded-lg border p-3 ${props.class || ""}`}
      >
        <div class="flex items-center justify-between">
          <span class="text-sm">
            <Trans
              key="content:_trn_selected_count"
              options={{ count: props.selectedRowsLength() }}
            />
          </span>
          <div class="flex items-center gap-2">
            <Show when={props.hasUpdates()}>
              <Button
                size="small"
                type="secondary"
                disabled={props.isInstanceLocked()}
                onClick={props.onUpdateSelected}
              >
                <div class="i-hugeicons:download-02" />
                <Trans key="content:_trn_update_selected" />
              </Button>
            </Show>
            <Button
              size="small"
              disabled={props.isInstanceLocked()}
              onClick={props.onDeleteSelected}
            >
              <div class="i-hugeicons:delete-02" />
              <Trans key="content:_trn_delete_selected" />
            </Button>
          </div>
        </div>
      </div>
    </Show>
  )
}
