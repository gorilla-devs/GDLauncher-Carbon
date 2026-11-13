/**
 * Where a deleted server's folder goes: `undefined` until the "Deletion
 * through Recycle Bin" setting is known.
 */
export type ServerDeletionTarget = "recycleBin" | "permanent" | undefined

export function serverDeletionTarget(
  deletionThroughRecycleBin: boolean | undefined
): ServerDeletionTarget {
  if (deletionThroughRecycleBin === undefined) return undefined
  return deletionThroughRecycleBin ? "recycleBin" : "permanent"
}

/**
 * Whether deleting a server in `status` stops it first. Only a server this
 * session started: one left running by an earlier session is refused by the
 * core until it is stopped.
 */
export function deletionStopsServer(status: string | undefined) {
  return status === "running"
}

/** The confirm button's label: it says everything the click does. */
export function serverDeleteLabelKey(
  stopsFirst: boolean,
  target: ServerDeletionTarget
) {
  if (target === "permanent") {
    return stopsFirst
      ? "instances:_trn_server_delete_stop_and_delete_permanently"
      : "instances:_trn_server_delete_permanently"
  }
  return stopsFirst
    ? "instances:_trn_server_delete_stop_and_delete"
    : "instances:_trn_instance_confirm_deletion.delete"
}
