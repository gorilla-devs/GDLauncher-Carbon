/* eslint-disable solid/no-innerhtml */
import { createEffect, createResource, Match, Suspense, Switch } from "solid-js"
import { Skeleton } from "@gd/ui"
import { parseToHtml } from "@/utils/modplatformDescriptionConverter"
import { FEUnifiedProjectID, MRFEProject } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { useParams } from "@solidjs/router"

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

const Overview = () => {
  const params = useParams()

  const addonId = params.id
  const platform = params.platform

  const unifiedProjectId =
    platform === "curseforge"
      ? ({
          type: "curseforge",
          id: parseInt(addonId)
        } satisfies FEUnifiedProjectID)
      : ({
          type: "modrinth",
          id: addonId
        } satisfies FEUnifiedProjectID)

  const description = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedGetProject",
      {
        projectId: unifiedProjectId
      }
    ]
  }))

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
