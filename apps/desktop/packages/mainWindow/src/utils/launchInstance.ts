import { rspc } from "@/utils/rspcClient"

/**
 * Starting a game, for every Play control. The core first renews the
 * account's sign-in when it has run out, which can take a few seconds:
 * `launching` is true until it answers, and a press in between is ignored.
 * What the core refuses to start by itself (too little memory, a sign-in
 * that could not be renewed) it asks about in a dialog of its own.
 */
export function useLaunchInstance() {
  const mutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))
  return {
    launching: () => mutation.isPending,
    launch: (id: number) => {
      if (mutation.isPending) return
      mutation.mutate({ id, skipMemoryCheck: false })
    }
  }
}
