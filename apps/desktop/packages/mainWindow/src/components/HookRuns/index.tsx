import { createEffect } from "solid-js"
import { toast } from "@gd/ui"
import { type NamespacedTranslationKey, useTransContext } from "@gd/i18n"
import type { FEHookKind, FEHookRun } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"

type Phase = FEHookRun["state"]["type"]

const TITLE: Record<Phase, Record<FEHookKind, NamespacedTranslationKey>> = {
  Running: {
    PreLaunch: "notifications:_trn_hook_running_pre",
    PostExit: "notifications:_trn_hook_running_post"
  },
  Finished: {
    PreLaunch: "notifications:_trn_hook_finished_pre",
    PostExit: "notifications:_trn_hook_finished_post"
  },
  Failed: {
    PreLaunch: "notifications:_trn_hook_failed_pre",
    PostExit: "notifications:_trn_hook_failed_post"
  }
}

/** Shell-level: one notice for each run of a pre-launch or post-exit hook.
 * It shows while the script runs and turns into how it ended: gone after a
 * moment when it worked, kept with the script's last line when it failed. */
export default function HookRuns() {
  const ctx = rspc.useContext()
  const [t] = useTransContext()
  const runs = rspc.createQuery(() => ({
    queryKey: ["instance.getHookRuns"]
  }))
  // The state each run was last shown in.
  const shown = new Map<number, Phase>()

  createEffect(() => {
    for (const run of runs.data ?? []) {
      const phase = run.state.type
      if (shown.get(run.id) === phase) continue
      shown.set(run.id, phase)
      const id = `hook-run-${run.id}`
      const title = t(TITLE[phase][run.kind], {
        instance: run.name,
        interpolation: { escapeValue: false }
      })
      if (run.state.type === "Running") {
        toast.loading(title, { id, duration: Infinity })
        continue
      }
      if (run.state.type === "Finished") {
        toast.success(title, { id, duration: 4000 })
      } else {
        toast.error(title, {
          id,
          duration: Infinity,
          description: run.state.value.summary
        })
      }
      ctx.client
        .mutation(["instance.acknowledgeHookRun", run.id])
        .catch((error) =>
          console.error("Could not acknowledge a hook run", error)
        )
    }
  })

  return null
}
