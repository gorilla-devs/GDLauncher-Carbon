import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  Collapsable,
  Input,
  Slider,
  Switch as GDSwitch,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { Show, createEffect, createMemo, untrack } from "solid-js"
import { useSearchParams } from "@solidjs/router"
import useSettingsJavaData from "./settings.java.data"
import { rspc } from "@/utils/rspcClient"
import { formatMemory, memoryMarks } from "@/utils/java/javaText"
import JavaArgumentsHint from "@/components/Java/JavaArgumentsHint"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"
import RightHandSide from "./components/RightHandSide"
import { MIN_MEMORY_MB, shownMemoryMb, typedMemoryMb } from "./javaMemory"
import Center from "./components/Center"
import YourJavas from "./java/YourJavas"
import CustomProfiles from "./java/CustomProfiles"
import {
  AdvisoryReviewContext,
  createAdvisoryReview
} from "./java/advisoryReview"
import { reviewTargets } from "./java/javaOverviewView"

const Java = () => {
  const [t, { getI18next }] = useTransContext()
  const routeData = useSettingsJavaData()
  const settings = routeData.settings
  const memory = (mb: number) =>
    formatMemory(mb, getI18next().language, (key) => t(key))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const memoryWarningDismissed = rspc.createQuery(() => ({
    queryKey: ["settings.getMemoryWarningDismissed"]
  }))

  const memoryWarningDismissedMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setMemoryWarningDismissed"]
  }))

  const mbTotalRAM = () =>
    Math.round(Number(routeData.totalRam.data) / 1024 / 1024)

  const initialJavaArgs = createMemo<{ value: string } | undefined>(
    (previous) => {
      if (previous !== undefined) return previous
      const loaded = settings.data
      return loaded === undefined ? undefined : { value: loaded.javaCustomArgs }
    }
  )

  // `?review=<advisoryId>&java=<id>…`, from the advisory toast's "Review",
  // with the Javas the toast counted.
  const [searchParams, setSearchParams] = useSearchParams<{
    review: string
    java: string | string[]
  }>()
  const review = createAdvisoryReview()

  createEffect(() => {
    const advisoryId = searchParams.review
    const overview = routeData.javaOverview.data
    if (!advisoryId || !overview) return

    const targets = reviewTargets(
      overview,
      advisoryId,
      [searchParams.java ?? []].flat(),
      routeData.javaOverview.isFetching
    )
    if (!targets) return

    untrack(() => {
      const reducedMotion =
        settings.data?.reducedMotion ||
        window.matchMedia("(prefers-reduced-motion: reduce)").matches
      review.start(targets, reducedMotion ? "instant" : "smooth")
      setSearchParams({ review: undefined, java: undefined }, { replace: true })
    })
  })

  return (
    <>
      <PageTitle>
        <Trans key="settings:_trn_java" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="java:_trn_java_memory_title" />
          </Title>
          <Center>
            <Slider
              min={MIN_MEMORY_MB}
              max={mbTotalRAM()}
              steps={1024}
              marks={memoryMarks(1024, mbTotalRAM(), memory)}
              value={shownMemoryMb(settings.data?.xmx)}
              tooltipFormat={memory}
              onChange={(val) => {
                settingsMutation.mutate({
                  xmx: {
                    Set: val
                  }
                })
              }}
            />
            <Input
              class="w-32"
              inputColor="bg-darkSlate-600"
              data-testid="java-memory-box"
              inputMode="numeric"
              icon={<span class="text-sm">{t("java:_trn_unit_mb")}</span>}
              value={shownMemoryMb(settings.data?.xmx)}
              onChange={(e) => {
                const typed = typedMemoryMb(e.currentTarget.value)
                const shown = shownMemoryMb(settings.data?.xmx)
                // The box shows what is in force at once: the minimum for a
                // lower number, and what it held for anything else.
                e.currentTarget.value = String(typed ?? shown ?? "")
                if (typed === null || typed === settings.data?.xmx) return
                settingsMutation.mutate({
                  xmx: {
                    Set: typed
                  }
                })
              }}
            />
          </Center>
          <Show
            when={settings.data?.xmx && settings.data.xmx > mbTotalRAM() * 0.8}
          >
            <div class="mt-2 flex items-center gap-2 text-sm text-yellow-500">
              <div class="i-hugeicons:alert-02 h-4 w-4 shrink-0" />
              <Trans key="java:_trn_ram_warning_high_allocation" />
            </div>
          </Show>
        </Row>
        <Row>
          <Title
            description={<Trans key="java:_trn_show_memory_warning_text" />}
          >
            <Trans key="java:_trn_show_memory_warning_title" />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={!memoryWarningDismissed.data}
              onChange={(e) => {
                memoryWarningDismissedMutation.mutate(!e.currentTarget.checked)
              }}
            />
          </RightHandSide>
        </Row>
        <Row class="flex-col items-stretch">
          <Title description={<JavaArgumentsHint />}>
            <Trans key="java:_trn_java_arguments_title" />
          </Title>
          <div class="flex items-center justify-center gap-4">
            <Input
              class="w-full"
              value={settings.data?.javaCustomArgs}
              onChange={(e) => {
                settingsMutation.mutate({
                  javaCustomArgs: {
                    Set: e.target.value
                  }
                })
              }}
            />
            <Tooltip>
              <TooltipTrigger
                type="button"
                class="b-0 rounded-md bg-transparent p-0 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
                aria-label={t("java:_trn_a11y_undo_java_args")}
                disabled={initialJavaArgs() === undefined}
                onClick={() => {
                  const baseline = initialJavaArgs()
                  if (baseline === undefined) return
                  settingsMutation.mutate({
                    javaCustomArgs: {
                      Set: baseline.value
                    }
                  })
                }}
              >
                <Button as="div" type="secondary" class="h-10" size="small">
                  <div class="i-hugeicons:arrow-turn-backward h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.undo" />
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger
                type="button"
                class="b-0 rounded-md bg-transparent p-0 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
                aria-label={t("java:_trn_a11y_reset_java_args")}
                onClick={() => {
                  settingsMutation.mutate({
                    javaCustomArgs: {
                      Set: ""
                    }
                  })
                }}
              >
                <Button as="div" type="secondary" class="h-10" size="small">
                  <div class="i-hugeicons:cancel-01 h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.reset" />
              </TooltipContent>
            </Tooltip>
          </div>
        </Row>
        <Row>
          <Title
            description={<Trans key="java:_trn_keep_javas_up_to_date_text" />}
          >
            <Trans key="java:_trn_keep_javas_up_to_date" />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={settings.data?.autoUpdateManagedJava}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoUpdateManagedJava: {
                    Set: e.currentTarget.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <div class="flex flex-col gap-2 py-6">
          <AdvisoryReviewContext.Provider value={review}>
            <YourJavas
              overview={routeData.javaOverview}
              installs={routeData.managedJavaInstalls.data ?? []}
            />
          </AdvisoryReviewContext.Provider>
          <Collapsable
            title={<Trans key="instances:_trn_instance_share.advanced" />}
            defaultOpened={false}
            noPadding
            size="small"
          >
            <CustomProfiles
              profiles={routeData.javaProfiles.data ?? []}
              overview={routeData.javaOverview.data}
            />
          </Collapsable>
        </div>
      </RowsContainer>
    </>
  )
}

export default Java
