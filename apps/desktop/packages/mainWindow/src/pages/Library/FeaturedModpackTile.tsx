import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { Skeleton } from "@gd/ui"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createResource,
  createSignal
} from "solid-js"

const HEXING_TALES_MODPACK_ID = 891604

const FeaturedModpackTile = () => {
  const navigator = useGDNavigate()
  const rspcContext = rspc.useContext()
  const [shouldShow, setShouldShow] = createSignal(true)

  const [hexingTales] = createResource(() => {
    return rspcContext.client
      .query([
        "modplatforms.curseforge.getMod",
        {
          modId: HEXING_TALES_MODPACK_ID
        }
      ])
      .catch(console.error)
  })

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const globalStore = useGlobalStore()
  const settings = () => globalStore.settings.data
  const instances = () => globalStore.instances.data

  createEffect(() => {
    if (!instances()) return
    if (!settings()) return

    for (const i of instances()!) {
      if (
        i.status.status === "valid" &&
        i.status.value.modpack?.type === "curseforge" &&
        i.status.value.modpack?.value?.project_id === HEXING_TALES_MODPACK_ID
      ) {
        setShouldShow(false)
        return
      }
    }

    setShouldShow(true)
  })

  return (
    <Show when={shouldShow()}>
      <>
        <div class="bg-darkSlate-400 h-24 w-px" />
        <Show when={settings()?.showFeatured}>
          <div
            class="w-70 outline-darkSlate-500 relative box-border h-24 overflow-hidden rounded-md outline-2 duration-200 ease-in-out"
            classList={{
              "group hover:outline hover:bg-darkSlate-700":
                !!hexingTales()?.data
            }}
            onClick={() => {
              navigator.navigate(`/addon/${HEXING_TALES_MODPACK_ID}/curseforge`)
            }}
          >
            <div
              class="z-1 text-lightSlate-900 hover:text-lightSlate-50 absolute right-2 top-2 opacity-50 duration-200 ease-in-out i-hugeicons:view h-4 w-4"
              onClick={(e) => {
                e.stopPropagation()
                settingsMutation.mutate({
                  showFeatured: {
                    Set: false
                  }
                })
              }}
            />
            <div class="absolute left-0 top-0 duration-200 ease-in-out group-hover:-translate-y-full">
              <Trans key="library:_trn_featured.try_featured_modpack" />
            </div>
            <Switch>
              <Match when={hexingTales()?.data}>
                <div class="relative h-full w-full">
                  <img
                    src={hexingTales()?.data.logo?.url}
                    class="group-hover:scale-130 absolute bottom-0 left-0 h-16 w-16 rounded-lg duration-200 ease-in-out group-hover:-translate-y-4 group-hover:translate-x-4"
                  />
                  <div class="absolute bottom-0 left-20 duration-200 ease-in-out group-hover:opacity-0">
                    <div class="text-nowrap text-xl font-bold">
                      {hexingTales()?.data.name}
                    </div>
                    <div class="text-lightSlate-700 text-sm">
                      <For each={hexingTales()?.data.authors}>
                        {(v) => <span>{v.name}</span>}
                      </For>
                    </div>
                  </div>

                  <div class="absolute left-40 top-1/2 -translate-y-1/2 translate-x-[150%] duration-200 ease-in-out group-hover:translate-x-0">
                    <Trans key="library:_trn_featured.show_more" />
                  </div>
                </div>
              </Match>
              <Match when={!hexingTales()?.data}>
                <div class="relative h-full w-full">
                  <div class="absolute bottom-0 left-0 w-full">
                    <Skeleton.featuredHomeTile />
                  </div>
                </div>
              </Match>
            </Switch>
          </div>
        </Show>
        <Show when={!settings()?.showFeatured}>
          <div
            class="text-lightSlate-900 hover:text-lightSlate-50 my-2 mr-2 opacity-50 duration-200 ease-in-out i-hugeicons:view-off-slash h-4 w-4"
            onClick={(e) => {
              e.stopPropagation()
              settingsMutation.mutate({
                showFeatured: {
                  Set: true
                }
              })
            }}
          />
        </Show>
      </>
    </Show>
  )
}

export default FeaturedModpackTile
