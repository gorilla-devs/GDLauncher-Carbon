import { Button } from "@gd/ui"
import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
import { Portal } from "solid-js/web"
import adSize from "@/utils/adhelper"

interface Props {
  selectedCount: () => number
  onClearSelection: () => void
  onDelete: () => void
}

export const SelectionActionBar = (props: Props) => {
  return (
    <Show when={props.selectedCount() > 0}>
      <Portal>
        <div
          class="fixed bottom-6 left-0 z-50 flex justify-center animate-popoverEnter"
          style={{ width: `calc(100vw - ${adSize.width}px)` }}
        >
          <div class="bg-darkSlate-800 border border-darkSlate-600 flex items-center gap-6 rounded-2xl px-5 py-3 shadow-lg shadow-darkSlate-900/50">
            {/* Selection count */}
            <span class="text-lightSlate-50 text-sm font-medium">
              <Trans
                key="instances:_trn_selected_count"
                options={{ count: props.selectedCount() }}
              />
            </span>

            {/* Delete button */}
            <Button size="small" type="secondary" onClick={props.onDelete}>
              <div class="i-hugeicons:delete-02 h-4 w-4" />
              <span>
                <Trans key="general:_trn_delete" />
              </span>
            </Button>

            {/* Close button */}
            <div
              class="text-darkSlate-300 i-hugeicons:cancel-01 hover:text-lightSlate-100 h-5 w-5 cursor-pointer press-effect active:scale-90"
              onClick={props.onClearSelection}
            />
          </div>
        </div>
      </Portal>
    </Show>
  )
}
