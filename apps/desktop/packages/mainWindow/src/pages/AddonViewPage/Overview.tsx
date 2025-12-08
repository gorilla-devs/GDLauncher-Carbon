import { Match, Suspense, Switch, useContext } from "solid-js"
import { Skeleton } from "@gd/ui"
import { parseToHtml } from "@/utils/modplatformDescriptionConverter"
import { AddonContext } from "."

const Description = () => {
  const mod = useContext(AddonContext)

  const description = () => {
    return parseToHtml(mod?.data?.fullDescriptionBody)
  }

  return (
    <Suspense fallback={<Skeleton.modpackOverviewPage />}>
      <div>
        <div
          class="w-full max-w-full overflow-hidden [&_img]:max-w-full [&_img]:h-auto break-words"
          // eslint-disable-next-line solid/no-innerhtml
          innerHTML={description()}
        />
      </div>
    </Suspense>
  )
}

const Overview = () => {
  const mod = useContext(AddonContext)

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
