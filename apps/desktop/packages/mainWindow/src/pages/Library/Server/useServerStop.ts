import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import { offersForceStop, stopClock } from "@/utils/serverStop"

/**
 * How the stop of a server is going, read every second while it is
 * stopping. The launcher waits for as long as the server takes, so this is
 * what tells the user it is still working, and Force stop is theirs to choose.
 */
export function useServerStop(id: () => number, stopping: () => boolean) {
  const modalsContext = useModal()
  const progress = rspc.createQuery(() => ({
    queryKey: ["server.getServerStopProgress", id()],
    enabled: stopping(),
    refetchInterval: stopping() ? 1000 : false,
    gcTime: 0
  }))
  const current = () => (stopping() ? (progress.data ?? null) : null)
  const seconds = () => current()?.stoppingSeconds
  return {
    progress: current,
    /** "0:53", or undefined until the first reading. */
    clock: () => {
      const value = seconds()
      return value == null ? undefined : stopClock(value)
    },
    canForce: () => offersForceStop(seconds()),
    /** Opens the confirmation; nothing is stopped until the user confirms. */
    askForceStop: (name: string) =>
      modalsContext?.openModal({ name: "forceStopServer" }, { id: id(), name })
  }
}
