import { Button } from "@gd/ui"
import { For, Show, createEffect, createSignal } from "solid-js"
import { Trans } from "@gd/i18n"
import Version from "./Version"
import { useRouteData } from "@solidjs/router"
import fetchData from "../../instance.data"
import { rspc } from "@/utils/rspcClient"
import { CFFEFile } from "@gd/core_module/bindings"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

const NoVersions = () => {
  return (
    <div class="min-h-90 flex h-full w-full items-center justify-center">
      <div class="flex flex-col items-center justify-center gap-6 text-center">
        <PlaceholderGorilla size={8} variant="Shrugging Gorilla - No Options" />
        <p class="text-lightSlate-700 max-w-100">
          <Trans
            key="content:_trn_modpack.no_versions_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain any other versions"
            }}
          />
        </p>
        <Button type="outline" size="medium">
          <Trans
            key="content:_trn_modpack.no_versions"
            options={{
              defaultValue: "No versions"
            }}
          />
        </Button>
      </div>
    </div>
  )
}

const Versions = () => {
  const [versions, setVersions] = createSignal<CFFEFile[]>([])
  const [mainFileId, setMainFileId] = createSignal<undefined | number>(
    undefined
  )
  const routeData: ReturnType<typeof fetchData> = useRouteData()

  const modId = () =>
    routeData.instanceDetails?.data?.modpack?.modpack?.type === "curseforge" &&
    routeData.instanceDetails.data?.modpack?.modpack.value?.project_id

  if (modId()) {
    const instanceDetails = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.curseforge.getMod",
        {
          modId: modId() as number
        }
      ]
    }))

    createEffect(() => {
      setMainFileId(instanceDetails.data?.data.mainFileId)
      if (instanceDetails.data?.data.latestFilesIndexes) {
        instanceDetails.data?.data.latestFiles.forEach((latestFile) => {
          setVersions((prev) => [...prev, latestFile])
        })
      }
    })
  }

  return (
    <div>
      <div class="h-full overflow-y-hidden">
        <Show
          when={versions().length > 0 && mainFileId()}
          fallback={<NoVersions />}
        >
          <For each={versions()}>
            {(props) => <Version version={props} mainFileId={mainFileId()!} />}
          </For>
        </Show>
      </div>
    </div>
  )
}

export default Versions
