import { Trans } from "@gd/i18n"
import { Show } from "solid-js"
import { DeletionTarget } from "@/utils/deletionTarget"
import {
  DeletionNoteBox,
  DeletionNoteLine,
  DeletionTargetLine
} from "./DeletionNote"

interface Props {
  /** Where the folder goes; nothing is said about it until it is known. */
  target: DeletionTarget
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
      <DeletionNoteBox testId="server-deletion-note">
        <Show when={props.stopsFirst}>
          <DeletionNoteLine
            tone="yellow"
            strong
            testId="server-deletion-running"
          >
            <Trans
              key={
                props.several
                  ? "instances:_trn_servers_delete_running"
                  : "instances:_trn_server_delete_running"
              }
            />
          </DeletionNoteLine>
        </Show>
        <DeletionTargetLine
          target={props.target}
          recycleBin={
            <Trans
              key={
                props.several
                  ? "instances:_trn_servers_delete_to_recycle_bin"
                  : "instances:_trn_server_delete_to_recycle_bin"
              }
            />
          }
          permanent={
            <Trans
              key={
                props.several
                  ? "instances:_trn_servers_delete_permanent"
                  : "instances:_trn_server_delete_permanent"
              }
            />
          }
        />
      </DeletionNoteBox>
    </Show>
  )
}

export default ServerDeletionNote
