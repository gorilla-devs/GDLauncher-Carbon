import { Progress, Spinner } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { createSignal, onMount, onCleanup, Match, Switch } from "solid-js"
import type { StepProps } from "."
import { rspc } from "@/utils/rspcClient"
import { setShaderInstallRunning } from "./state"

type StepName = "modloader" | "loader" | "shader"

const Installing = (props: StepProps) => {
  const [stepName, setStepName] = createSignal<StepName>("loader")
  const [stepIndex, setStepIndex] = createSignal(0)
  const [stepCount, setStepCount] = createSignal(1)
  const [error, setError] = createSignal<string | null>(null)
  const [destroyed, setDestroyed] = createSignal(false)

  // Lock the backdrop close before any await fires. The ModalsManager reads
  // this via the registry's function-form `preventClose`, so without it a
  // backdrop click during install could dismiss the modal mid-pipeline.
  setShaderInstallRunning(true)

  onCleanup(() => {
    setDestroyed(true)
    setShaderInstallRunning(false)
  })

  const ctx = rspc.useContext()

  const waitForTask = async (taskId: number): Promise<void> => {
    while (!destroyed()) {
      const task = await ctx.client.query(["vtask.getTask", taskId])
      if (task === null) return
      if (task.progress.type === "Failed") {
        const cause = task.progress.value.cause[0]?.display
        throw new Error(cause ?? "Task failed")
      }
      if (task.progress.type === "Known" && task.progress.value >= 1.0) {
        return
      }
      await new Promise((r) => setTimeout(r, 400))
    }
  }

  const installModloader = async (instanceId: number): Promise<number> => {
    return ctx.client.mutation([
      "instance.installFabricLoaderDefault",
      instanceId
    ])
  }

  const installShaderLoader = async (
    instanceId: number,
    projectId: string
  ): Promise<number> => {
    return ctx.client.mutation([
      "instance.installLatestMod",
      {
        instance_id: instanceId,
        mod_source: { Modrinth: projectId }
      }
    ])
  }

  const installShader = async (): Promise<number | null> => {
    const data = props.data
    if (data.installLatest && data.latestModSource) {
      return ctx.client.mutation([
        "instance.installLatestMod",
        {
          instance_id: data.instanceId,
          mod_source: data.latestModSource
        }
      ])
    }
    if (!data.installLatest && data.modSource) {
      return ctx.client.mutation([
        "instance.installMod",
        {
          instance_id: data.instanceId,
          mod_source: data.modSource,
          install_deps: data.replacesMod === undefined,
          replaces_mod: data.replacesMod ?? null
        }
      ])
    }
    return null
  }

  const run = async () => {
    const plan = props.installPlan()
    const recommendation = props.data.recommendation

    const steps: { name: StepName; run: () => Promise<number | null> }[] = []

    if (!plan.fileOnly) {
      if (recommendation.kind === "RequiresModloader") {
        steps.push({
          name: "modloader",
          run: () => installModloader(props.data.instanceId)
        })
        steps.push({
          name: "loader",
          run: () =>
            installShaderLoader(
              props.data.instanceId,
              recommendation.loader_modrinth_id
            )
        })
      } else if (recommendation.kind === "RecommendLoader") {
        steps.push({
          name: "loader",
          run: () =>
            installShaderLoader(
              props.data.instanceId,
              recommendation.loader_modrinth_id
            )
        })
      }
    }

    steps.push({
      name: "shader",
      run: () => installShader()
    })

    setStepCount(steps.length)

    let lastTaskId: number | null = null
    for (let i = 0; i < steps.length; i++) {
      if (destroyed()) return
      setStepIndex(i)
      setStepName(steps[i].name)
      try {
        const taskId = await steps[i].run()
        if (taskId !== null) {
          lastTaskId = taskId
          await waitForTask(taskId)
        }
      } catch (e) {
        if (destroyed()) return
        setError((e as Error)?.message ?? "Install failed")
        return
      }
    }

    if (destroyed()) return
    props.data.onComplete?.(lastTaskId)
    props.setStep("done")
  }

  onMount(() => {
    void run()
  })

  return (
    <div class="w-130 flex h-60 flex-col items-center justify-around">
      <Spinner class="h-12 w-12" />
      <div class="flex flex-col items-center gap-2 text-center">
        <h3 class="m-0 text-lg">
          <Switch>
            <Match when={stepName() === "modloader"}>
              <Trans key="content:_trn_shader_loader_installing_modloader" />
            </Match>
            <Match when={stepName() === "loader"}>
              <Trans key="content:_trn_shader_loader_installing_loader" />
            </Match>
            <Match when={stepName() === "shader"}>
              <Trans key="content:_trn_shader_loader_installing_shader" />
            </Match>
          </Switch>
        </h3>
        <p class="text-darkSlate-100 m-0 text-xs">
          <Trans
            key="content:_trn_shader_loader_step_progress"
            options={{
              current: stepIndex() + 1,
              total: stepCount()
            }}
          />
        </p>
      </div>
      <Progress value={Math.round(((stepIndex() + 1) / stepCount()) * 100)} />
      {error() && <p class="text-red-400 m-0 text-xs">{error()}</p>}
    </div>
  )
}

export default Installing
