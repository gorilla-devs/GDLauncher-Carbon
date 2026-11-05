import { createEffect } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"

/** Shell-level: says once that servers missing from the library, for
 * example after a database reset, were put back from the `server.json` the
 * launcher keeps in each server's folder. */
export default function RestoredServers() {
  const ctx = rspc.useContext()
  const [t] = useTransContext()
  const restored = rspc.createQuery(() => ({
    queryKey: ["server.getRestoredServers"]
  }))
  let told = 0

  createEffect(() => {
    const count = restored.data ?? 0
    if (count <= 0) {
      told = 0
      return
    }
    if (count === told) return
    told = count
    toast.info(t("notifications:_trn_servers_restored", { count }), {
      duration: 10000,
      description: t("notifications:_trn_servers_restored_hint", { count })
    })
    ctx.client
      .mutation(["server.acknowledgeRestoredServers"])
      .catch((error) =>
        console.error("Could not acknowledge the restored servers", error)
      )
  })

  return null
}
