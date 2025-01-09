/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router"
import { createEffect, createResource, Match, Suspense, Switch } from "solid-js"
import { Skeleton } from "@gd/ui"
import fetchData from "../modpack.overview"
import { parseToHtml } from "@/utils/modplatformDescriptionConverter"
import { MRFEProject } from "@gd/core_module/bindings"

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData()

  const Description = () => {
    const [data, { refetch }] = createResource(async () => {
      const params: [string | undefined, "html" | "markdown"] = [
        undefined,
        "html"
      ]

      if (routeData.isCurseforge) {
        params[0] = routeData.modpackDescription?.data?.data
        params[1] = "html"
      } else {
        params[0] = routeData.modpackDetails.data?.body
        params[1] = "markdown"
      }

      return parseToHtml(params[0], params[1])
    })

    createEffect(() => {
      const _1 = routeData.modpackDescription?.data?.data
      const _2 = (routeData.modpackDetails.data as MRFEProject)?.body
      refetch()
    })

    return (
      <Suspense fallback={<Skeleton.modpackOverviewPage />}>
        <div>
          <div class="w-full max-w-full overflow-hidden" innerHTML={data()} />
        </div>
      </Suspense>
    )
  }

  return (
    <Switch fallback={<Skeleton.modpackOverviewPage />}>
      <Match when={!routeData.modpackDescription?.isLoading}>
        <Description />
      </Match>
      <Match when={routeData.modpackDescription?.isLoading}>
        <Skeleton.modpackOverviewPage />
      </Match>
    </Switch>
  )
}

export default Overview
