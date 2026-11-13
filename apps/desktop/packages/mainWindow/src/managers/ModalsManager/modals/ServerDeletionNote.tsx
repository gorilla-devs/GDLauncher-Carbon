import { Trans } from "@gd/i18n"
import { Show } from "solid-js"
import { ServerDeletionTarget } from "@/utils/serverDeletion"

interface Props {
  /** Where the folder goes; nothing is said about it until it is known. */
  target: ServerDeletionTarget
  /** A running server is stopped first. */
  stopsFirst: boolean
  /** More than one server is being deleted. */
  several?: boolean
}

/**
 * What a server delete does beyond removing the tile: stops a running
 * server, and either moves the folder to the Recycle Bin or erases it.
 */
function ServerDeletionNote(props: Props) {
  return (
    <Show when={props.stopsFirst || props.target}>
      <div
        class="bg-darkSlate-900 text-lightSlate-300 flex flex-col gap-2 rounded-lg p-3 text-sm"
        data-testid="server-deletion-note"
      >
        <Show when={props.stopsFirst}>
          <div
            class="flex items-start gap-2"
            data-testid="server-deletion-running"
          >
            <div class="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-yellow-400" />
            <span class="text-lightSlate-50 font-bold">
              <Trans
                key={
                  props.several
                    ? "instances:_trn_servers_delete_running"
                    : "instances:_trn_server_delete_running"
                }
              />
            </span>
          </div>
        </Show>
        <Show when={props.target === "recycleBin"}>
          <div
            class="flex items-start gap-2"
            data-testid="server-deletion-recycle-bin"
          >
            <div class="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-green-400" />
            <span>
              <Trans
                key={
                  props.several
                    ? "instances:_trn_servers_delete_to_recycle_bin"
                    : "instances:_trn_server_delete_to_recycle_bin"
                }
              />
            </span>
          </div>
        </Show>
        <Show when={props.target === "permanent"}>
          <div
            class="flex items-start gap-2"
            data-testid="server-deletion-permanent"
          >
            <div class="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-red-400" />
            <span>
              <Trans
                key={
                  props.several
                    ? "instances:_trn_servers_delete_permanent"
                    : "instances:_trn_server_delete_permanent"
                }
              />
            </span>
          </div>
        </Show>
      </div>
    </Show>
  )
}

export default ServerDeletionNote
