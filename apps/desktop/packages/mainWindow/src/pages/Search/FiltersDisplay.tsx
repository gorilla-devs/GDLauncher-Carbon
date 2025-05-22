import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { CategoryIcon, getInstanceImageUrl } from "@/utils/instances"
import { rspc } from "@/utils/rspcClient"
import { Badge } from "@gd/ui"
import { useSearchParams } from "@solidjs/router"
import { For, Show } from "solid-js"
import DefaultImg from "/assets/images/default-instance-img.png"

export default function FiltersDisplay() {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()
  const [searchParams, setSearchParams] = useSearchParams()
  const instanceId = () => parseInt(searchParams.instanceId, 10)

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()]
  }))

  return (
    <Show
      when={searchContext?.searchQuery().categories?.length || instanceId()}
    >
      <div class="px-6 pt-6">
        <div class="flex items-center gap-4">
          <div>Active Filters:</div>
          <Show when={instanceId()}>
            <div
              class="border-1 border-darkSlate-500 group relative box-border flex h-10 items-center justify-between gap-4 overflow-hidden rounded-lg border-solid px-4"
              style={{
                "background-image":
                  instanceDetails.data?.iconRevision && instanceId()
                    ? `url("${getInstanceImageUrl(
                        instanceId(),
                        instanceDetails.data?.iconRevision
                      )}")`
                    : `url("${DefaultImg}")`
              }}
              onClick={() => {
                setSearchParams({
                  ...searchParams,
                  instanceId: undefined
                })
                searchContext?.setSearchQuery({
                  ...searchContext.searchQuery(),
                  modloaders: null,
                  gameVersions: null,
                  categories: null
                })
              }}
            >
              <div class="from-darkSlate-700 absolute inset-0 z-0 bg-gradient-to-r from-50%" />
              <div class="from-darkSlate-700 absolute inset-0 z-0 bg-gradient-to-t" />
              <div class="z-10 flex items-center gap-4">
                <div
                  class="h-6 w-6 bg-cover bg-center"
                  style={{
                    "background-image": instanceDetails.data?.iconRevision
                      ? `url("${getInstanceImageUrl(
                          instanceId(),
                          instanceDetails.data?.iconRevision
                        )}")`
                      : `url("${DefaultImg}")`
                  }}
                />
                <h2 class="m-0">{instanceDetails.data?.name}</h2>
              </div>
              <i class="i-ri:close-fill text-lightSlate-700 group-hover:text-lightSlate-50 h-5 w-5 transition-colors" />
            </div>
          </Show>
          <For each={searchContext?.searchQuery().categories}>
            {(category) => {
              // Not the best... but at this point we have no way of knowing which platform the category belongs to
              const categoryData =
                globalStore.categories.data?.curseforge[category as number] ??
                globalStore.categories.data?.modrinth[category as string]

              console.log(categoryData)

              return (
                <Badge
                  variant="secondary"
                  class="group flex h-10 items-center gap-4 px-4"
                  onClick={(e) => {
                    e.preventDefault()
                    e.stopPropagation()
                    searchContext?.setSearchQuery((prev) => ({
                      ...prev,
                      categories:
                        prev.categories?.filter((c) => c !== category) ?? null
                    }))
                  }}
                >
                  <CategoryIcon
                    type={categoryData?.icon?.type}
                    value={categoryData?.icon?.value}
                  />
                  {categoryData?.name}
                  <i class="i-ri:close-fill text-lightSlate-700 group-hover:text-lightSlate-50 h-5 w-5 transition-colors" />
                </Badge>
              )
            }}
          </For>
        </div>
      </div>
    </Show>
  )
}
