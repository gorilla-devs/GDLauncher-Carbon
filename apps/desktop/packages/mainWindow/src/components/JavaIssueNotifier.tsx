import { createEffect, untrack } from "solid-js"
import { useLocation } from "@solidjs/router"
import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import { createJavaIssuePrompts } from "@/utils/javaIssues"

/** This session's prompts: each crash is asked about once. */
const prompts = createJavaIssuePrompts()

/** Opens the crash help dialog when a game crashes on a Java older than a
 *  mod needs. Renders nothing. */
const JavaIssueNotifier = () => {
  const modals = useModal()
  const location = useLocation()

  const instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }))

  createEffect(() => {
    // Not over the login screen: no game runs from there.
    if (location.pathname === "/") return
    const list = instances.data
    if (!list) return

    untrack(() => {
      for (const prompt of prompts.take(list)) {
        modals?.openModal({ name: "javaIssue" }, prompt)
      }
    })
  })

  return null
}

export default JavaIssueNotifier
