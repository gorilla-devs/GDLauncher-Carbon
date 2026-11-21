import { queryClient, rspc } from "@/utils/rspcClient"
import { PREFERENCES_KEY } from "./tileSize"

/**
 * `themes.setPreferences`. Its answer is the merged preferences, so the cache
 * takes it at once instead of waiting for the core's invalidation to come
 * back.
 */
export function createPreferencesMutation(opts: { onError?: () => void } = {}) {
  return rspc.createMutation(() => ({
    mutationKey: ["themes.setPreferences"],
    onSuccess: (data) => {
      queryClient.setQueryData(PREFERENCES_KEY, data)
    },
    onError: () => opts.onError?.()
  }))
}
