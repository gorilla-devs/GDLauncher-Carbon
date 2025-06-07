import { For, JSX } from "solid-js"
import { cn } from "../util"

const Skeleton = (props: {
  class?: string
  style?: string | JSX.CSSProperties
}) => {
  return (
    <div
      class={cn("w-1/3 h-4 rounded-md bg-darkSlate-500", props.class)}
      style={props.style}
    />
  )
}

const SidebarInstance = () => {
  return (
    <div class="flex gap-2 px-4 py-2">
      <div class="bg-darkSlate-500 h-10 w-10 rounded-lg" />
      <div class="space-between flex flex-col gap-2">
        <div class="bg-darkSlate-500 h-4 w-32 rounded-md" />
        <div class="bg-darkSlate-500 h-4 w-32 rounded-md" />
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
  return <div class="bg-darkSlate-500 h-10 w-10 rounded-lg px-4 py-2" />
}

const Instance = () => {
  return (
    <div class="flex flex-col gap-2">
      <div class="w-38 h-38 bg-darkSlate-500 rounded-lg" />
      <div class="space-between flex flex-col gap-2">
        <div class="bg-darkSlate-500 h-4 w-32 rounded-md" />
        <div class="bg-darkSlate-500 h-4 w-32 rounded-md" />
      </div>
    </div>
  )
}

Skeleton.instance = Instance

Skeleton.instances = () => {
  return (
    <div class="flex flex-col gap-4">
      <div class="bg-darkSlate-500 h-10 w-full rounded-lg" />
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

Skeleton.news = () => {
  return <div class="bg-darkSlate-500 mb-5 h-24 w-full rounded-lg" />
}

const Modpack = () => {
  return (
    <div class="bg-darkSlate-500 box-border flex h-40 w-full justify-between gap-4 rounded-xl p-4">
      <div class="h-30 w-30 bg-darkSlate-500 select-none rounded-xl" />
      <div class="space-between flex flex-1 flex-col gap-2">
        <div class="bg-darkSlate-500 h-4 w-full rounded-md" />
        <div class="bg-darkSlate-500 h-4 w-full rounded-md" />
        <div class="bg-darkSlate-500 h-4 w-1/2 rounded-md" />
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
      <div class="bg-darkSlate-500 h-2 w-full rounded-md" />
      <div class="bg-darkSlate-500 h-2 w-1/2 rounded-md" />
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
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-full rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-full rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-full rounded-xl" />
      <div class="mt-5 flex flex-wrap gap-4">
        <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
        <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
      </div>
    </div>
  )
}

Skeleton.modpackScreenshotsPage = () => {
  return (
    <div class="mt-5 flex flex-wrap gap-4">
      <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
      <div class="bg-darkSlate-500 h-44 w-72 rounded-xl" />
    </div>
  )
}

Skeleton.modpackChangelogPage = () => {
  return (
    <div class="flex w-full flex-col gap-2">
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
      <div class="bg-darkSlate-500 mt-5 h-3 w-1/4 rounded-xl" />
      <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
    </div>
  )
}

Skeleton.modpackSidebarCategories = () => {
  return (
    <div class="mt-4 flex w-full flex-col gap-4 py-2">
      <For each={new Array(16)}>
        {() => (
          <div class="flex items-center gap-2">
            <div class="bg-darkSlate-500 h-5 w-5 rounded-xl" />
            <div class="bg-darkSlate-500 h-3 w-1/2 rounded-xl" />
          </div>
        )}
      </For>
    </div>
  )
}

Skeleton.select = () => {
  return <div class="w-31	bg-darkSlate-500 h-12 rounded-full" />
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
        <div class="bg-darkSlate-500 h-10 w-40 rounded-full" />
        <div class="bg-darkSlate-500 h-10 w-40 rounded-full" />
        <div class="bg-darkSlate-500 h-10 w-40 rounded-full" />
      </div>
      <div class="flex flex-col gap-2">
        <div class="w-100 bg-darkSlate-500 h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 h-36 rounded-xl" />
        <div class="w-100 bg-darkSlate-500 h-36 rounded-xl" />
      </div>
    </div>
  )
}

Skeleton.featuredHomeTile = () => {
  return (
    <div class="flex w-full flex-col gap-4">
      <div class="flex h-fit w-full items-end gap-4">
        <div class="bg-darkSlate-500 h-16 w-16 rounded-lg" />
        <div class="flex h-full flex-col gap-2">
          <div class="w-30 bg-darkSlate-500 h-6 rounded-full" />
          <div class="bg-darkSlate-500 h-4 w-20 rounded-full" />
        </div>
      </div>
    </div>
  )
}

const SearchListItem = () => {
  return (
    <div class="my-1 overflow-hidden rounded-md px-4">
      <div class="relative flex h-full cursor-pointer gap-2 overflow-hidden rounded-md border border-transparent p-1.5">
        <div class="relative z-10 flex w-full items-center gap-4">
          <div class="bg-darkSlate-500 h-16 w-16 rounded-md" />
          <div class="w-7/10 flex flex-col gap-2">
            <div class="truncate text-left text-xl font-medium">
              <div class="bg-darkSlate-500 h-7 w-3/4 rounded-md" />
            </div>
            <div class="text-lightSlate-700 truncate text-left text-sm">
              <div class="bg-darkSlate-500 h-5 w-full rounded-md" />
            </div>
            <div class="flex gap-2">
              <div class="bg-darkSlate-500 h-5.5 w-16 rounded-md" />
              <div class="bg-darkSlate-500 h-5.5 w-20 rounded-md" />
            </div>
          </div>
          <div class="ml-auto flex items-center">
            <div class="relative flex items-center">
              <div class="flex items-center gap-2">
                <div class="text-lightSlate-700 text-sm">
                  <div class="bg-darkSlate-500 h-5.5 w-14 rounded-md" />
                </div>
                <div class="bg-darkSlate-500 h-5.5 w-12 rounded" />
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
    <div class="flex w-full flex-col gap-1">
      <For each={new Array(12)}>{() => <SearchListItem />}</For>
    </div>
  )
}

export { Skeleton }
