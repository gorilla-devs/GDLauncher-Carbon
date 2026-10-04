import type { FEServerState } from "@gd/core_module/bindings"
import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import {
  recoveredServerAction,
  recoveredServerPid
} from "@/utils/recoveredServer"

/**
 * Runs the main action of a server left behind by an earlier session: Stop
 * right away where the server can save first, otherwise the dialog that
 * asks the user to confirm a force stop or clearing the record.
 */
export function useRecoveredServerAction() {
  const modalsContext = useModal()
  const stopRecoveredMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopRecoveredServer"]
  }))

  const run = (server: { id: number; name: string; state: FEServerState }) => {
    const action = recoveredServerAction(server.state)
    if (action === "stop") {
      stopRecoveredMutation.mutate(server.id)
    } else if (action) {
      modalsContext?.openModal(
        { name: "recoveredServerProcess" },
        {
          id: server.id,
          name: server.name,
          action,
          processId: recoveredServerPid(server.state)
        }
      )
    }
  }

  return { run, isPending: () => stopRecoveredMutation.isPending }
}
