import { For, JSX } from "solid-js"
import { cn } from "../util"

const Skeleton = (props: {
  class?: string
  style?: string | JSX.CSSProperties
}) => {
  return (
    <div
      class={cn(
        "w-1/3 h-4 rounded-md bg-darkSlate-500 skeleton-shimmer",
        props.class
      )}
      style={props.style}
    />
  )
}

const SidebarInstance = () => {
  return (
    <div class="flex gap-2 px-4 py-2">
      <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-10 rounded-lg skeleton-shimmer" />
      <div class="space-between flex flex-col gap-2">
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-32 rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-32 rounded-md skeleton-shimmer" />
      </div>
    </div>
  )
}

Skeleton.sidebarInstance = SidebarInstance

Skeleton.sidebarInstances = () => {
  return (
    <div class="mt-10 flex flex-col gap-2">
      <For each={new Array(4)}>{() => <SidebarInstance />}</For>
    </div>
  )
}

Skeleton.sidebarInstanceSmall = () => {
  return (
    <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-10 rounded-lg px-4 py-2 skeleton-shimmer" />
  )
}

const Instance = () => {
  return (
    <div class="flex flex-col gap-2">
      <div class="w-38 h-38 bg-darkSlate-500 skeleton-shimmer rounded-lg skeleton-shimmer" />
      <div class="space-between flex flex-col gap-2">
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-32 rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-32 rounded-md skeleton-shimmer" />
      </div>
    </div>
  )
}

Skeleton.instance = Instance

Skeleton.instances = () => {
  return (
    <div class="flex flex-col gap-4">
      <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-full rounded-lg skeleton-shimmer" />
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
      <div class="flex gap-4">
        <For each={new Array(10)}>{() => <Instance />}</For>
      </div>
    </div>
  )
}

const NewsItem = () => {
  return (
    <div class="cursor-pointer">
      <div class="bg-darkSlate-500 skeleton-shimmer aspect-video w-full rounded-lg skeleton-shimmer" />
      <div class="p-4 space-y-2">
        <div class="bg-darkSlate-500 skeleton-shimmer h-6 w-3/4 rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-2/3 rounded-md skeleton-shimmer" />
      </div>
    </div>
  )
}

Skeleton.newsCarousel = () => {
  return (
    <div class="bg-darkSlate-500 skeleton-shimmer relative h-84 overflow-hidden rounded-lg mb-5 skeleton-shimmer">
      <div class="absolute bottom-0 left-0 p-8 space-y-2">
        <div class="bg-darkSlate-400 skeleton-shimmer h-8 w-80 rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-400 skeleton-shimmer h-6 w-96 rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-400 skeleton-shimmer h-4 w-24 rounded-md mt-4 skeleton-shimmer" />
      </div>
      <div class="absolute bottom-4 left-1/2 -translate-x-1/2 flex gap-2">
        <div class="bg-darkSlate-400 skeleton-shimmer h-2 w-2 rounded-full opacity-100 skeleton-shimmer" />
        <div class="bg-darkSlate-400 skeleton-shimmer h-2 w-2 rounded-full opacity-30 skeleton-shimmer" />
        <div class="bg-darkSlate-400 skeleton-shimmer h-2 w-2 rounded-full opacity-30 skeleton-shimmer" />
      </div>
    </div>
  )
}

Skeleton.news = () => {
  return (
    <div class="flex flex-col gap-4 p-6">
      <div class="bg-darkSlate-500 skeleton-shimmer h-8 w-48 rounded-md skeleton-shimmer" />
      <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        <For each={new Array(6)}>{() => <NewsItem />}</For>
      </div>
    </div>
  )
}

const Modpack = () => {
  return (
    <div class="bg-darkSlate-500 skeleton-shimmer box-border flex h-40 w-full justify-between gap-4 rounded-xl p-4 skeleton-shimmer">
      <div class="h-30 w-30 bg-darkSlate-500 skeleton-shimmer select-none rounded-xl skeleton-shimmer" />
      <div class="space-between flex flex-1 flex-col gap-2">
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md skeleton-shimmer" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-1/2 rounded-md skeleton-shimmer" />
      </div>
    </div>
  )
}

Skeleton.modpack = Modpack

Skeleton.modpacksList = () => {
  return (
    <div class="box-border flex w-full flex-col gap-2 px-4">
      <Modpack />
      <Modpack />
      <Modpack />
      <Modpack />
    </div>
  )
}
const ModpackVersion = () => {
  return (
    <div class="box-border flex w-1/2 flex-col justify-between gap-4 rounded-xl p-4">
      <div class="bg-darkSlate-500 skeleton-shimmer h-2 w-full rounded-md skeleton-shimmer" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-2 w-1/2 rounded-md skeleton-shimmer" />
    </div>
  )
}
Skeleton.modpackVersionList = () => {
  return (
    <div class="flex w-full flex-col gap-2">
      <ModpackVersion />
      <ModpackVersion />
      <ModpackVersion />
      <ModpackVersion />
    </div>
  )
}
Skeleton.modpackOverviewPage = () => {
  return (
    <div class="flex w-full flex-col gap-2">
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-full rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-full rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-full rounded-xl" />
      <div class="mt-5 flex flex-wrap gap-4">
        <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
      </div>
    </div>
  )
}

Skeleton.modpackScreenshotsPage = () => {
  return (
    <div class="mt-5 flex flex-wrap gap-4">
      <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-44 w-72 rounded-xl" />
    </div>
  )
}

Skeleton.modpackChangelogPage = () => {
  return (
    <div class="flex w-full flex-col gap-2">
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
    </div>
  )
}

Skeleton.modpackSidebarCategories = () => {
  return (
    <div class="mt-4 flex w-full flex-col gap-4 py-2">
      <For each={new Array(16)}>
        {() => (
          <div class="flex items-center gap-2">
            <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-5 rounded-xl" />
            <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-xl" />
          </div>
        )}
      </For>
    </div>
  )
}

Skeleton.select = () => {
  return <div class="w-31	bg-darkSlate-500 skeleton-shimmer h-12 rounded-full" />
}

Skeleton.filters = () => {
  return (
    <div class="flex gap-2 pb-4">
      <Skeleton.select />
      <Skeleton.select />
      <Skeleton.select />
      <Skeleton.select />
    </div>
  )
}

Skeleton.explorer = () => {
  return (
    <div class="flex flex-col gap-4">
      <div class="flex gap-4">
        <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-40 rounded-full" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-40 rounded-full" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-40 rounded-full" />
      </div>
      <div class="flex flex-col gap-2">
        <div class="w-100 bg-darkSlate-500 skeleton-shimmer h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 skeleton-shimmer h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 skeleton-shimmer h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 skeleton-shimmer h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 skeleton-shimmer h-36 rounded-xl" />
      </div>
    </div>
  )
}

Skeleton.featuredHomeTile = () => {
  return (
    <div class="flex w-full flex-col gap-4">
      <div class="flex h-fit w-full items-end gap-4">
        <div class="bg-darkSlate-500 skeleton-shimmer h-16 w-16 rounded-lg" />
        <div class="flex h-full flex-col gap-2">
          <div class="w-30 bg-darkSlate-500 skeleton-shimmer h-6 rounded-full" />
          <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-20 rounded-full" />
        </div>
      </div>
    </div>
  )
}

const SearchListItem = () => {
  return (
    <div class="my-2 overflow-hidden rounded-md">
      <div class="relative flex h-full cursor-pointer gap-2 overflow-hidden rounded-md border border-transparent px-8 py-2">
        <div class="relative z-10 flex w-full items-center gap-4">
          <div class="bg-darkSlate-500 skeleton-shimmer h-16 w-16 rounded-md" />
          <div class="w-7/10 flex flex-col gap-2">
            <div class="truncate text-left text-xl font-medium">
              <div class="bg-darkSlate-500 skeleton-shimmer h-7 w-3/4 rounded-md" />
            </div>
            <div class="text-lightSlate-700 truncate text-left text-sm">
              <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-full rounded-md" />
            </div>
            <div class="flex gap-2">
              <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-16 rounded-md" />
              <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-20 rounded-md" />
            </div>
          </div>
          <div class="ml-auto flex items-center">
            <div class="relative flex items-center">
              <div class="flex items-center gap-2">
                <div class="text-lightSlate-700 text-sm">
                  <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-14 rounded-md" />
                </div>
                <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-12 rounded" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

Skeleton.searchListItem = SearchListItem

Skeleton.searchList = () => {
  return (
    <div class="flex w-full flex-col gap-4 px-6">
      <For each={new Array(12)}>{() => <SearchListItem />}</For>
    </div>
  )
}

const AddonVersionsTableRow = () => {
  return (
    <div class="border-darkSlate-700 hover:bg-darkSlate-700/30 grid min-h-[70px] grid-cols-[4fr_130px_100px_120px_150px] gap-4 border-b py-2">
      <div class="flex items-center gap-3">
        <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-5 rounded" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-48 rounded-md" />
      </div>
      <div class="flex items-center">
        <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-20 rounded-md" />
      </div>
      <div class="flex items-center">
        <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-16 rounded-md" />
      </div>
      <div class="flex items-center">
        <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-16 rounded-md" />
      </div>
      <div class="flex items-center justify-end">
        <div class="bg-darkSlate-500 skeleton-shimmer h-8 w-24 rounded-md" />
      </div>
    </div>
  )
}

Skeleton.addonVersionsTable = () => {
  return (
    <div class="flex w-full flex-col">
      <For each={new Array(6)}>{() => <AddonVersionsTableRow />}</For>
    </div>
  )
}

const LogsListItem = () => {
  return (
    <div class="flex items-center gap-2 px-4 py-2 hover:bg-darkSlate-600">
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-4 rounded" />
      <div class="flex flex-1 flex-col gap-1">
        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-3/4 rounded-md" />
        <div class="bg-darkSlate-500 skeleton-shimmer h-3 w-1/2 rounded-md" />
      </div>
    </div>
  )
}

Skeleton.logsList = () => {
  return (
    <div class="flex w-full flex-col">
      <For each={new Array(10)}>{() => <LogsListItem />}</For>
    </div>
  )
}

const InstanceSettingsRow = () => {
  return (
    <div class="flex flex-col gap-2 py-3">
      <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-40 rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-full rounded-md" />
    </div>
  )
}

Skeleton.instanceSettings = () => {
  return (
    <div class="flex w-full flex-col gap-4 p-4">
      <div class="bg-darkSlate-500 skeleton-shimmer h-8 w-48 rounded-md" />
      <For each={new Array(6)}>{() => <InstanceSettingsRow />}</For>
    </div>
  )
}

Skeleton.contentArea = () => {
  return (
    <div class="flex w-full flex-col gap-4 p-6">
      <div class="bg-darkSlate-500 skeleton-shimmer h-10 w-64 rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-3/4 rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer mt-4 h-48 w-full rounded-lg" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded-md" />
      <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-5/6 rounded-md" />
    </div>
  )
}

Skeleton.instanceGrid = () => {
  return (
    <div class="grid grid-cols-2 gap-4 p-4 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      <For each={new Array(10)}>{() => <Instance />}</For>
    </div>
  )
}

export { Skeleton }
