import { retryCapturedAddonFailure } from "@/components/AddonFailures/controller"
import { resolveTaskPoll } from "@/components/ModDownloadButton/hooks/resolveTaskPoll"
import { Button, Progress, Spinner } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { getTaskTranslationKey } from "@gd/i18n/helpers"
import {
  createSignal,
  createEffect,
  onMount,
  onCleanup,
  Match,
  Switch,
  Show
} from "solid-js"
import type {
  FETask,
  FETaskOutcome,
  FESubtask,
  Translation,
  ShaderLoaderVerification
} from "@gd/core_module/bindings"
import type { StepProps } from "."
import { extractErrorDisplay, rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"
import { setShaderInstallRunning } from "./state"
import { formatBytes } from "@/utils/formatBytes"

import { runShaderSteps } from "./pipeline"

type StepName = "modloader" | "loader" | "shader"

const getTranslationArgs = (translation: Translation) => {
  if ("args" in translation) {
    return translation.args
  }
  return {}
}

const Installing = (props: StepProps) => {
  const [t] = useTransContext()
  const contextKey = () =>
    JSON.stringify([
      props.data.instanceId,
      props.data.recommendation,
      props.data.installLatest,
      props.data.modSource,
      props.data.latestModSource,
      props.data.replacesMod,
      props.installPlan().fileOnly
    ])
  const acceptedContext = contextKey()
  const data = {
    ...props.data,
    modSource: props.data.modSource
      ? JSON.parse(JSON.stringify(props.data.modSource))
      : undefined,
    latestModSource: props.data.latestModSource
      ? JSON.parse(JSON.stringify(props.data.latestModSource))
      : undefined,
    recommendation: { ...props.data.recommendation }
  }
  const plan = { ...props.installPlan() }
  let busy = false
  let completed = false
  const modalsContext = useModal()
  const [stepName, setStepName] = createSignal<StepName>("loader")
  const [stepIndex, setStepIndex] = createSignal(0)
  const [stepCount, setStepCount] = createSignal(1)
  const [error, setError] = createSignal<string | null>(null)
  const [unresolvedRequest, setUnresolvedRequest] = createSignal(false)
  const [destroyed, setDestroyed] = createSignal(false)
  const [currentTask, setCurrentTask] = createSignal<FETask | null>(null)

  // Lock the backdrop close before any await fires. The ModalsManager reads
  // this via the registry's function-form `preventClose`, so without it a
  // backdrop click during install could dismiss the modal mid-pipeline.
  setShaderInstallRunning(true)

  onCleanup(() => {
    setDestroyed(true)
    setShaderInstallRunning(false)
  })

  createEffect(() => {
    if (contextKey() !== acceptedContext) {
      setDestroyed(true)
      setShaderInstallRunning(false)
      setError(t("content:_trn_shader_loader_context_changed"))
    }
  })
  const ctx = rspc.useContext()
  interface Stage {
    name: StepName
    run: () => Promise<number | null>
    task?: number | null
    succeeded?: boolean
    done?: boolean
    failed?: boolean
    uncertain?: boolean
    recoverFrom?: number
  }
  let activeStage: Stage | undefined

  const waitForTask = async (taskId: number): Promise<void> => {
    while (!destroyed()) {
      let outcome: FETaskOutcome
      try {
        outcome = await ctx.client.query(["vtask.getTaskOutcome", taskId])
      } catch {
        // A read failure does not retire the accepted attempt or offer a retry.
        await new Promise((r) => setTimeout(r, 200))
        continue
      }
      if (destroyed()) return
      if (outcome.type === "Running" || outcome.type === "Failed")
        setCurrentTask(outcome.value)
      const { action } = resolveTaskPoll(outcome, null, false)
      if (action.kind === "completed") {
        if (activeStage) activeStage.succeeded = true
        return
      }
      if (action.kind === "failed" || action.kind === "dismissed") {
        if (activeStage) {
          activeStage.failed =
            outcome.type === "Failed" || outcome.type === "Dismissed"
          activeStage.uncertain = !activeStage.failed
        }
        throw new Error(
          action.kind === "failed"
            ? action.message
            : t("notifications:_trn_addon_attempt_dismissed")
        )
      }
      await new Promise((r) => setTimeout(r, 200))
    }
  }

  const installModloader = async (instanceId: number): Promise<number> => {
    return ctx.client.mutation([
      "instance.installFabricLoaderDefault",
      instanceId
    ])
  }

  let loaderVerification: ShaderLoaderVerification | undefined
  const installShaderLoader = async (instanceId: number): Promise<number> => {
    const result = await ctx.client.mutation([
      "instance.ensureShaderLoader",
      instanceId
    ])
    loaderVerification = result.verification
    return result.task_id
  }

  const installShader = async (): Promise<number | null> => {
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

  const buildSteps = () => {
    const recommendation = data.recommendation
    const steps: Stage[] = []

    if (!plan.fileOnly) {
      if (recommendation.kind === "RequiresModloader") {
        steps.push({
          name: "modloader",
          run: () => installModloader(data.instanceId)
        })
        steps.push({
          name: "loader",
          run: () => installShaderLoader(data.instanceId)
        })
      } else if (recommendation.kind === "RecommendLoader") {
        steps.push({
          name: "loader",
          run: () => installShaderLoader(data.instanceId)
        })
      }
    }

    steps.push({
      name: "shader",
      run: () => installShader()
    })

    return steps
  }
  const steps = buildSteps()
  setStepCount(steps.length)
  const run = async () => {
    if (destroyed() || completed) return
    try {
      await runShaderSteps(
        steps.map((step, i) => async () => {
          setStepIndex(i)
          setStepName(step.name)
          setCurrentTask(null)
          activeStage = step
          if (step.done || step.succeeded) return null
          if (step.task !== undefined) return step.task
          try {
            step.task =
              step.recoverFrom !== undefined
                ? await ctx.client.mutation([
                    "instance.prepareInstance",
                    {
                      instance: data.instanceId,
                      expected_failed_task: step.recoverFrom,
                      expected_mc_version:
                        data.recommendation.kind === "RequiresModloader"
                          ? data.recommendation.mc_version
                          : ""
                    }
                  ])
                : await step.run()
            step.uncertain = false
            if (step.task === null) step.succeeded = true
            return step.task
          } catch (error) {
            step.uncertain = true
            setUnresolvedRequest(step.recoverFrom === undefined)
            if (step.recoverFrom === undefined) props.markUnresolvedRequest()
            throw error
          }
        }),
        waitForTask,
        async () => {
          if (loaderVerification)
            return ctx.client.query([
              "instance.verifyShaderLoader",
              loaderVerification
            ])
          const result = await ctx.client.query([
            "instance.checkShaderRequirements",
            data.instanceId
          ])
          return result.kind === "LoaderPresent"
        },
        plan.fileOnly,
        destroyed,
        (task) => {
          if (completed || destroyed()) return
          completed = true
          setShaderInstallRunning(false)
          props.complete(task ?? steps.at(-1)?.task ?? null)
          modalsContext?.closeModal()
        },
        async (index) => {
          if (
            steps[index].name === "loader" &&
            (!loaderVerification ||
              !(await ctx.client.query([
                "instance.verifyShaderLoader",
                loaderVerification
              ])))
          ) {
            throw new Error(t("content:_trn_shader_loader_verification_failed"))
          }
          steps[index].done = !!steps[index].succeeded
        }
      )
    } catch (e) {
      if (destroyed()) return
      setError(
        extractErrorDisplay(e).trim() ||
          t("content:_trn_shader_loader_install_failed")
      )
      setShaderInstallRunning(false)
    }
  }

  const cancel = () => {
    if (completed) return
    completed = true
    setDestroyed(true)
    setShaderInstallRunning(false)
    props.complete(null)
    modalsContext?.closeModal()
  }

  const retry = async () => {
    if (busy || destroyed() || completed) return
    busy = true
    setError(null)
    setShaderInstallRunning(true)
    try {
      const stage = activeStage
      // A rejected RPC may have started retained work without returning its ID.
      // Repeating that mutation would duplicate an operation we cannot observe.
      if (
        stage?.uncertain &&
        stage.task === undefined &&
        stage.recoverFrom === undefined
      ) {
        throw new Error(t("content:_trn_shader_loader_uncertain"))
      }
      if (loaderVerification) {
        const enabled = await ctx.client.query([
          "instance.verifyShaderLoader",
          loaderVerification
        ])
        if (destroyed()) return
        if (
          steps.some((step) => step.name === "loader" && step.done) &&
          !enabled
        ) {
          throw new Error(t("content:_trn_shader_loader_verification_failed"))
        }
      }
      const failed =
        stage?.failed && typeof stage.task === "number" ? stage.task : null
      await retryCapturedAddonFailure(
        failed,
        (id) => ctx.client.query(["vtask.getTaskOutcome", id]),
        (id) => ctx.client.mutation(["vtask.dismissTask", id]),
        async () => {
          if (destroyed()) return
          if (failed !== null && stage) {
            stage.failed = false
            stage.recoverFrom = stage.name === "modloader" ? failed : undefined
            stage.task = undefined
          }
          await run()
        }
      )
    } catch (error) {
      if (!destroyed())
        setError(
          extractErrorDisplay(error).trim() ||
            t("content:_trn_shader_loader_install_failed")
        )
      setShaderInstallRunning(false)
    } finally {
      busy = false
    }
  }

  onMount(() => {
    busy = true
    void run().finally(() => {
      busy = false
    })
  })

  const taskProgressFraction = () => {
    const task = currentTask()
    if (!task) return 0
    if (task.progress.type === "Known") return task.progress.value
    return 0
  }

  const overallPercent = () => {
    const total = stepCount()
    if (total === 0) return 0
    return Math.round(((stepIndex() + taskProgressFraction()) / total) * 100)
  }

  const activeSubtask = (): FESubtask | undefined =>
    currentTask()?.active_subtasks[0]

  const subtaskDetail = () => {
    const sub = activeSubtask()
    if (!sub) return null
    const p = sub.progress
    if (typeof p === "object" && "download" in p) {
      return `${formatBytes(p.download.downloaded)} / ${formatBytes(p.download.total)}`
    }
    if (typeof p === "object" && "item" in p) {
      return `${p.item.current} / ${p.item.total}`
    }
    return null
  }

  return (
    <Show
      when={!error()}
      fallback={
        <div class="w-130 flex h-72 flex-col items-center justify-center gap-4 text-center">
          <div class="i-hugeicons:alert-02 text-3xl text-red-400" />
          <p class="text-red-400 m-0 text-sm">{error()}</p>
          <Show when={unresolvedRequest()}>
            <p class="m-0 text-sm">
              {t("content:_trn_shader_loader_uncertain")}
            </p>
          </Show>
          <div class="mt-2 flex gap-2">
            <Button type="secondary" onClick={cancel}>
              <Trans key="content:_trn_shader_loader_cancel" />
            </Button>
            <Show when={!unresolvedRequest() && !destroyed()}>
              <Button type="primary" onClick={retry}>
                <div class="i-hugeicons:refresh h-4 w-4" />
                <Trans key="general:_trn_retry" />
              </Button>
            </Show>
          </div>
        </div>
      }
    >
      <div class="w-130 flex h-72 flex-col items-center justify-around">
        <Spinner class="h-12 w-12" />
        <div class="flex flex-col items-center gap-1 text-center">
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
          <Show when={activeSubtask()} keyed>
            {(sub) => (
              <p class="text-darkSlate-100 m-0 text-sm">
                {t(
                  getTaskTranslationKey(sub.name.translation),
                  getTranslationArgs(sub.name)
                )}
              </p>
            )}
          </Show>
          <Show when={subtaskDetail()} keyed>
            {(detail) => (
              <p class="text-darkSlate-200 m-0 font-mono text-xs">{detail}</p>
            )}
          </Show>
        </div>
        <div class="flex w-full flex-col items-center gap-1">
          <Progress value={overallPercent()} />
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
      </div>
    </Show>
  )
}

export default Installing
