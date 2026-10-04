import { createEffect, For } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { describeTaskWarnings, type WarningToast } from "./describe"

const WarningLines = (props: { content: WarningToast }) => (
  <div class="flex flex-col gap-1">
    <For each={props.content.lines}>{(line) => <span>{line}</span>}</For>
    <span>{props.content.hint}</span>
  </div>
)

/** Shell-level: shows what a finished task left out (for example a required
 * dependency that has no compatible version), whichever view started it. */
export default function TaskWarnings() {
  const ctx = rspc.useContext()
  const [t] = useTransContext()
  const pending = rspc.createQuery(() => ({
    queryKey: ["vtask.getTaskWarnings"]
  }))
  const shown = new Set<number>()

  createEffect(() => {
    for (const entry of pending.data ?? []) {
      if (shown.has(entry.id)) continue
      shown.add(entry.id)
      const content = describeTaskWarnings(entry, (key, options) =>
        t(key, options)
      )
      if (content) {
        toast.warning(content.title, {
          duration: Infinity,
          description: <WarningLines content={content} />
        })
      }
      ctx.client
        .mutation(["vtask.acknowledgeTaskWarnings", entry.id])
        .catch((error) =>
          console.error("Could not acknowledge task warnings", error)
        )
    }
  })

  return null
}
