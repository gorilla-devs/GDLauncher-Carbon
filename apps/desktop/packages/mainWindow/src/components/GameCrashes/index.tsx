import { createEffect } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { describeGameCrash } from "./describe"

/** Shell-level: says that a game crashed or did not start, wherever the
 * player is in the app, with the way to that instance's log. */
export default function GameCrashes() {
  const ctx = rspc.useContext()
  const navigator = useGDNavigate()
  const [t] = useTransContext()
  const pending = rspc.createQuery(() => ({
    queryKey: ["instance.getGameCrashes"]
  }))
  const shown = new Set<number>()

  createEffect(() => {
    for (const crash of pending.data ?? []) {
      if (shown.has(crash.id)) continue
      shown.add(crash.id)
      const content = describeGameCrash(crash, (key, options) =>
        t(key, options)
      )
      toast.error(content.title, {
        duration: Infinity,
        description: content.description,
        action: {
          label: t("notifications:_trn_game_open_log"),
          onClick: () => navigator.navigate(`/library/${crash.instance}/logs`)
        }
      })
      ctx.client
        .mutation(["instance.acknowledgeGameCrash", crash.id])
        .catch((error) =>
          console.error("Could not acknowledge a game crash", error)
        )
    }
  })

  return null
}
