import { For, JSX, Suspense, onMount, useContext } from "solid-js"
import { Skeleton } from "@gd/ui"
import { Props as RowContainerProps } from "@/components/Browser/RowContainer"
import { InstanceDetails, Mod } from "@gd/core_module/bindings"
import { Trans } from "@gd/i18n"
import { VersionRowTypeData } from "../InfiniteScrollVersionsQueryWrapper"
import { AddonContext } from "@/pages/AddonViewPage"

interface Props {
  virtualVersions: any
  versions: VersionRowTypeData[]
  totalVirtualHeight: number
  measureElement: (_el: HTMLDivElement) => void
  instanceId?: number
  installedMod?: { id: string; remoteId: string }
  instanceMods?: Mod[]
  instanceDetails?: InstanceDetails
  isLoading: boolean
  children: (_: RowContainerProps) => JSX.Element
  type: "modpack" | "mod"
}

const MainContainer = (props: Props) => {
  const mod = useContext(AddonContext)

  const gridCols = "grid-cols-[5fr_130px_130px_100px_50px_200px]"

  return (
    <Suspense fallback={<Skeleton.modpackVersionList />}>
      <div class="w-full">
        <div class={`mb-8 grid ${gridCols}`}>
          <div>
            <Trans key="browser_table_headers.name" />
          </div>
          <div>
            <Trans key="browser_table_headers.published" />
          </div>
          <div>
            <Trans key="browser_table_headers.downloads" />
          </div>
          <div>
            <Trans key="browser_table_headers.type" />
          </div>
          <div>
            <Trans key="browser_table_headers.details" />
          </div>
        </div>
        <div
          class="w-full"
          style={{
            height: `${props.totalVirtualHeight}px`,
            position: "relative"
          }}
        >
          <For each={props.virtualVersions}>
            {(modFile: any) => {
              return (
                <div
                  data-index={modFile.index}
                  ref={(el) => {
                    onMount(() => {
                      props.measureElement(el)
                    })
                  }}
                  class={`grid ${gridCols}`}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: 0,
                    width: "100%",
                    height: `${modFile.size}px`,
                    transform: `translateY(${modFile.start}px)`
                  }}
                >
                  <props.children
                    project={mod?.data}
                    installedFile={props.installedMod}
                    modVersion={props.versions[modFile.index]}
                    instanceId={props.instanceId}
                    type={props.type}
                    instanceMods={props.instanceMods}
                    instanceDetails={props.instanceDetails}
                  />
                </div>
              )
            }}
          </For>
        </div>
      </div>
    </Suspense>
  )
}

export default MainContainer
