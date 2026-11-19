import { createEffect, For, Show } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import { useModal } from "@/managers/ModalsManager"
import {
  changeVersionRefusalText,
  isChangeVersionCancelled
} from "@/managers/ModalsManager/modals/ModPackVersionUpdate/changeVersionRefusal"
import {
  describeTaskWarnings,
  type WarningAction,
  type WarningToast
} from "./describe"

const WarningLines = (props: { content: WarningToast }) => (
  <div class="flex flex-col gap-1">
    <For each={props.content.lines}>{(line) => <span>{line}</span>}</For>
    <Show when={props.content.hint}>
      <span>{props.content.hint}</span>
    </Show>
  </div>
)

/** Shell-level: shows what a finished task left out (for example a required
 * dependency that has no compatible version), whichever view started it. */
export default function TaskWarnings() {
  const ctx = rspc.useContext()
  const modals = useModal()
  const [t] = useTransContext()
  const pending = rspc.createQuery(() => ({
    queryKey: ["vtask.getTaskWarnings"]
  }))
  const shown = new Set<number>()

  const run = (action: WarningAction) => {
    if (action.kind === "repair") {
      modals?.openModal(
        { name: "repairModpack" },
        { id: action.instance, name: action.name, isServer: false }
      )
      return
    }
    ctx.client
      .mutation([
        "instance.changeModpack",
        { instance: action.instance, modpack: action.modpack }
      ])
      .catch((error) => {
        if (isChangeVersionCancelled(error)) return
        const refusal = changeVersionRefusalText(error)
        toast.error(t("instances:_trn_change_version_failed"), {
          description: !refusal
            ? extractErrorDisplay(error)
            : "params" in refusal
              ? t(refusal.key, refusal.params)
              : t(refusal.key)
        })
      })
  }

  createEffect(() => {
    for (const entry of pending.data ?? []) {
      if (shown.has(entry.id)) continue
      shown.add(entry.id)
      const content = describeTaskWarnings(entry, (key, options) =>
        t(key, options)
      )
      if (content) {
        const action = content.action
        toast.warning(content.title, {
          duration: Infinity,
          description: <WarningLines content={content} />,
          ...(action
            ? {
                action: {
                  label:
                    action.kind === "repair"
                      ? t("instances:_trn_instance_settings.repair")
                      : t("notifications:_trn_pack_update_try_again"),
                  onClick: () => run(action)
                }
              }
            : {})
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
