import { rspc } from "@/utils/rspcClient"

/** For an instance that waits its turn to install or start: the name of the
 * instance it waits for, and the way out of the queue. Asks the core only
 * while the instance is queued. */
export function useInstallQueue(
  instanceId: () => number,
  isQueued: () => boolean
) {
  const turn = rspc.createQuery(() => ({
    queryKey: ["instance.getInstallTurn"],
    enabled: isQueued()
  }))
  const leaveMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.leaveQueue"]
  }))
  return {
    behind: () => (isQueued() ? turn.data?.name : undefined),
    leave: () => leaveMutation.mutate(instanceId())
  }
}
