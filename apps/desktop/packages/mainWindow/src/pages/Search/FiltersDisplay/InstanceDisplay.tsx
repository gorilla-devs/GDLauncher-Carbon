import useSearchContext from "@/components/SearchInputContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { rspc } from "@/utils/rspcClient"
import { Show } from "solid-js"
import DefaultImg from "/assets/images/default-instance-img.png"
import { FilterBadge } from "./FilterBadge"

export default function InstanceDisplay() {
  const searchContext = useSearchContext()

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: [
      "instance.getInstanceDetails",
      searchContext?.selectedInstanceId() ?? null
    ]
  }))

  return (
    <Show when={searchContext?.selectedInstanceId()}>
      <FilterBadge
        class="border-1 border-darkSlate-500 relative box-border flex h-10 items-center justify-between gap-4 overflow-hidden rounded-lg border-solid px-4"
        style={{
          "background-image":
            instanceDetails.data?.iconRevision &&
            searchContext?.selectedInstanceId()
              ? `url("${getInstanceImageUrl(
                  searchContext?.selectedInstanceId(),
                  instanceDetails.data?.iconRevision
                )}")`
              : `url("${DefaultImg}")`
        }}
        onClick={() => {
          searchContext?.setSelectedInstanceId(undefined)
          searchContext?.setSearchQuery({
            ...searchContext.searchQuery(),
            modloaders: null,
            gameVersions: null,
            projectType: "modpack"
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
                    searchContext?.selectedInstanceId() ?? 0,
                    instanceDetails.data?.iconRevision
                  )}")`
                : `url("${DefaultImg}")`
            }}
          />
          <h2 class="m-0">{instanceDetails.data?.name}</h2>
        </div>
      </FilterBadge>
    </Show>
  )
}
