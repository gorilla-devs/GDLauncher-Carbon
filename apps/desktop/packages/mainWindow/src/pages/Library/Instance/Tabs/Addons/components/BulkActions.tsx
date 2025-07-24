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
              key="instance.selected_count"
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
                <div class="i-ri:download-2-fill" />
                <Trans key="instance.update_selected" />
              </Button>
            </Show>
            <Button
              size="small"
              disabled={props.isInstanceLocked()}
              onClick={props.onDeleteSelected}
            >
              <div class="i-ri:delete-bin-2-fill" />
              <Trans key="instance.delete_selected" />
            </Button>
          </div>
        </div>
      </div>
    </Show>
  )
}
