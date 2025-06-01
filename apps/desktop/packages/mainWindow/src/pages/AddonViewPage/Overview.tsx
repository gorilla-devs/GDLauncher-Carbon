import { Match, Suspense, Switch, useContext } from "solid-js"
import { Skeleton } from "@gd/ui"
import { parseToHtml } from "@/utils/modplatformDescriptionConverter"
import { ModContext } from "."

const Description = () => {
  const mod = useContext(ModContext)

  const description = () => {
    return parseToHtml(mod?.data?.fullDescriptionBody)
  }

  return (
    <Suspense fallback={<Skeleton.modpackOverviewPage />}>
      <div>
        <div
          class="w-full max-w-full overflow-hidden"
          // eslint-disable-next-line solid/no-innerhtml
          innerHTML={description()}
        />
      </div>
    </Suspense>
  )
}

const Overview = () => {
  const mod = useContext(ModContext)

  return (
    <Switch fallback={<Skeleton.modpackOverviewPage />}>
      <Match when={!mod?.isLoading}>
        <Description />
      </Match>
      <Match when={mod?.isLoading}>
        <Skeleton.modpackOverviewPage />
      </Match>
    </Switch>
  )
}

export default Overview
