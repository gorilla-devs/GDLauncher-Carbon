/* eslint-disable solid/no-innerhtml */
import { useParams, useRouteData } from "@solidjs/router"
import {
  Match,
  Show,
  Suspense,
  Switch,
  createEffect,
  createSignal,
  createMemo
} from "solid-js"
import { createAsyncEffect } from "@/utils/asyncEffect"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import fetchData from "./changelog.data"
import { CFFEFile, CFFEFileIndex } from "@gd/core_module/bindings"
import { sortArrayByGameVersion } from "@/utils/mods"
import { format, formatDistanceToNowStrict } from "date-fns"

const ChangelogCard = (props: {
  content: string
  type: "html" | "text"
  platform: "curseforge" | "modrinth"
  releaseDate?: string
}) => {
  return (
    <div class="bg-darkSlate-700 border-darkSlate-600 overflow-hidden rounded-xl border shadow-lg">
      <div class="bg-darkSlate-750 border-darkSlate-600 flex flex-col gap-3 border-b p-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-3">
          <div class="h-6 w-1 shrink-0 rounded-full bg-blue-500" />
          <div>
            <h3 class="text-lightSlate-50 text-sm font-semibold sm:text-base">
              Release Notes
            </h3>
            <Show
              when={props.releaseDate}
              fallback={
                <p class="text-lightSlate-600 text-xs">
                  No release date available
                </p>
              }
            >
              <div class="text-lightSlate-600 flex items-center gap-2 text-xs">
                <div class="i-hugeicons:calendar-01 shrink-0 h-4 w-4" />
                <span>{format(new Date(props.releaseDate!), "PPP")}</span>
                <span class="text-lightSlate-700">•</span>
                <span class="text-lightSlate-500">
                  {formatDistanceToNowStrict(new Date(props.releaseDate!), {
                    addSuffix: true
                  })}
                </span>
              </div>
            </Show>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <Show when={props.platform === "curseforge"}>
            <div class="flex items-center gap-1.5 rounded-full bg-orange-500/10 px-2 py-1 text-xs text-orange-500">
              <div class="i-simple-icons:curseforge h-3 w-3 shrink-0" />
              <span class="font-medium">CurseForge</span>
            </div>
          </Show>
          <Show when={props.platform === "modrinth"}>
            <div class="flex items-center gap-1.5 rounded-full bg-green-500/10 px-2 py-1 text-xs text-green-500">
              <div class="i-simple-icons:modrinth h-3 w-3 shrink-0" />
              <span class="font-medium">Modrinth</span>
            </div>
          </Show>
        </div>
      </div>

      <div class="p-4 sm:p-6">
        <Show when={props.type === "html"}>
          <div
            class="prose prose-invert prose-sm sm:prose-base prose-headings:text-lightSlate-100
                   prose-headings:font-semibold prose-p:text-lightSlate-300
                   prose-p:leading-relaxed prose-p:mb-4 prose-a:text-blue-400
                   prose-a:no-underline hover:prose-a:text-blue-300 prose-strong:text-lightSlate-200
                   prose-strong:font-semibold prose-ul:text-lightSlate-300
                   prose-ol:text-lightSlate-300 prose-li:my-1
                   prose-li:leading-relaxed prose-code:text-pink-400
                   prose-code:bg-darkSlate-600 prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-pre:bg-darkSlate-600
                   prose-pre:border prose-pre:border-darkSlate-500 prose-pre:rounded-lg prose-pre:overflow-x-auto prose-blockquote:border-l-blue-500
                   prose-blockquote:bg-darkSlate-600 prose-blockquote:p-4 prose-blockquote:rounded-r-lg prose-hr:border-darkSlate-600
                   prose-table:text-sm
                   prose-table:border-collapse prose-th:border prose-th:border-darkSlate-500 prose-th:bg-darkSlate-600 prose-th:p-2 prose-td:border
                   prose-td:border-darkSlate-500 prose-td:p-2 max-w-none"
            innerHTML={props.content}
          />
        </Show>
        <Show when={props.type === "text"}>
          <pre class="text-lightSlate-300 bg-darkSlate-600 border-darkSlate-500 overflow-x-auto whitespace-pre-wrap rounded-lg border p-3 font-mono text-xs leading-relaxed sm:p-4 sm:text-sm">
            {props.content}
          </pre>
        </Show>
      </div>
    </div>
  )
}

const EnhancedChangelogSkeleton = () => {
  return (
    <div class="w-full space-y-6">
      <div class="bg-darkSlate-700 border-darkSlate-600 overflow-hidden rounded-xl border">
        <div class="bg-darkSlate-750 border-darkSlate-600 flex items-center justify-between border-b p-4">
          <div class="flex items-center gap-3">
            <div class="bg-darkSlate-500 h-6 w-1 animate-pulse rounded-full" />
            <div>
              <div class="bg-darkSlate-500 mb-1 h-5 w-24 animate-pulse rounded" />
              <div class="bg-darkSlate-500 h-3 w-32 animate-pulse rounded" />
            </div>
          </div>
          <div class="bg-darkSlate-500 h-6 w-20 animate-pulse rounded-full" />
        </div>
        <div class="space-y-4 p-6">
          <div class="bg-darkSlate-500 h-4 w-3/4 animate-pulse rounded" />
          <div class="bg-darkSlate-500 h-4 w-full animate-pulse rounded" />
          <div class="bg-darkSlate-500 h-4 w-2/3 animate-pulse rounded" />
          <div class="bg-darkSlate-500 h-4 w-5/6 animate-pulse rounded" />
          <div class="bg-darkSlate-500 h-4 w-1/2 animate-pulse rounded" />
          <div class="bg-darkSlate-500 h-4 w-4/5 animate-pulse rounded" />
        </div>
      </div>
    </div>
  )
}

const EmptyChangelogState = () => {
  return (
    <div class="flex flex-col items-center justify-center px-4 py-12 text-center sm:px-6 sm:py-16">
      <div class="mb-6">
        <PlaceholderGorilla size={8} variant="Reading Gorilla - Clipboard" />
      </div>
      <h3 class="text-lightSlate-200 mb-2 text-lg font-semibold sm:text-xl">
        No changelog available
      </h3>
      <p class="text-lightSlate-600 max-w-md text-sm leading-relaxed sm:text-base">
        This version doesn't have any changelog information available. Try
        selecting a different version or check back later.
      </p>
      <div class="bg-darkSlate-700 border-darkSlate-600 mt-6 w-full max-w-md rounded-lg border p-3 text-left sm:mt-8 sm:p-4">
        <div class="text-lightSlate-400 mb-2 flex items-center gap-2 text-sm">
          <div class="i-hugeicons:information-circle shrink-0 h-5 w-5" />
          <span class="font-medium">Tip</span>
        </div>
        <p class="text-lightSlate-600 text-xs leading-relaxed sm:text-sm">
          Some mod authors don't provide detailed changelogs for every release.
          You can check the mod's project page for more information about
          updates.
        </p>
      </div>
    </div>
  )
}

const Changelog = () => {
  const params = useParams()
  const rspcContext = rspc.useContext()

  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const lastFile = () =>
    routeData.isCurseforge &&
    routeData.modpackDetails?.data?.data.latestFiles[
      routeData.modpackDetails?.data?.data.latestFiles.length - 1
    ]

  const [options, setOptions] = createSignal<string[]>([])
  const [optionLabels, setOptionLabels] = createSignal<Record<string, string>>(
    {}
  )
  const [fileId, setFileId] = createSignal<number | string | undefined>(
    undefined
  )
  const [changeLog, setChangelog] = createSignal<string | undefined>(undefined)
  const [releaseDate, setReleaseDate] = createSignal<string | undefined>(
    undefined
  )
  const [isLoadingChangelog, setIsLoadingChangelog] =
    createSignal<boolean>(false)

  createEffect(() => {
    if (!routeData.modpackDetails.data) return
    if (!routeData.isCurseforge) {
      if (routeData.modrinthProjectVersions.data) {
        setFileId(routeData.modrinthProjectVersions.data[0].id)
        setChangelog(undefined)
        setReleaseDate(undefined)
        setIsLoadingChangelog(false)

        const opts = routeData.modrinthProjectVersions.data.map(
          (file) => file.id
        )
        const labels = Object.fromEntries(
          routeData.modrinthProjectVersions.data.map((file) => [
            file.id,
            file.version_number
          ])
        )
        setOptions(opts)
        setOptionLabels(labels)
      }
    } else {
      const sortedVersions = sortArrayByGameVersion(
        routeData.modpackDetails.data?.data.latestFilesIndexes
      )
      setChangelog(undefined)
      setReleaseDate(undefined)
      setIsLoadingChangelog(false)

      const opts = (sortedVersions as CFFEFileIndex[]).map((file) =>
        file.fileId.toString()
      )
      const labels = Object.fromEntries(
        (sortedVersions as CFFEFileIndex[]).map((file) => [
          file.fileId.toString(),
          file.filename
        ])
      )
      setOptions(opts)
      setOptionLabels(labels)
    }
  })

  createAsyncEffect((isStale) => {
    const modpackId = parseInt(params.id, 10)
    const currentFileId = fileId()
    const currentLastFile = lastFile()
    const isCurseforge = routeData.isCurseforge

    if (isCurseforge) {
      if (
        currentFileId !== undefined ||
        (currentLastFile && currentLastFile.id !== undefined)
      ) {
        setIsLoadingChangelog(true)
        setChangelog(undefined)
        setReleaseDate(undefined)

        const targetFileId =
          parseInt(currentFileId as string, 10) ||
          (currentLastFile as CFFEFile).id

        rspcContext.client
          .query([
            "modplatforms.curseforge.getModFileChangelog",
            {
              modId: modpackId,
              fileId: targetFileId
            }
          ])
          .then((changelogQuery) => {
            // Check if parameters haven't changed during async operation
            if (!isStale()) {
              setChangelog(changelogQuery.data)
              const fileData =
                routeData.modpackDetails.data?.data.latestFiles.find(
                  (file) => file.id === targetFileId
                ) ||
                routeData.modpackDetails.data?.data.latestFilesIndexes.find(
                  (file) => file.fileId === targetFileId
                )

              if (fileData && "fileDate" in fileData) {
                setReleaseDate(fileData.fileDate)
              } else {
                setReleaseDate(undefined)
              }
              setIsLoadingChangelog(false)
            }
          })
          .catch((e) => {
            console.error(e)
            if (!isStale()) {
              setChangelog(undefined)
              setReleaseDate(undefined)
              setIsLoadingChangelog(false)
            }
          })
      }
    }
  })

  createAsyncEffect((isStale) => {
    const currentFileId = fileId()
    const isCurseforge = routeData.isCurseforge

    if (!isCurseforge) {
      if (currentFileId !== undefined) {
        setIsLoadingChangelog(true)
        setChangelog(undefined)
        setReleaseDate(undefined)

        rspcContext.client
          .query(["modplatforms.modrinth.getVersion", currentFileId as string])
          .then((changelogQuery) => {
            // Check if parameters haven't changed during async operation
            if (!isStale()) {
              if (changelogQuery?.changelog) {
                setChangelog(changelogQuery.changelog)
              }
              if (changelogQuery?.date_published) {
                setReleaseDate(changelogQuery.date_published)
              } else {
                setReleaseDate(undefined)
              }
              setIsLoadingChangelog(false)
            }
          })
          .catch((err) => {
            console.error(err)
            if (!isStale()) {
              setChangelog(undefined)
              setReleaseDate(undefined)
              setIsLoadingChangelog(false)
            }
          })
      }
    }
  })

  const isLoading = createMemo(() => isLoadingChangelog())

  const hasContent = createMemo(() => changeLog() && changeLog()?.trim() !== "")

  return (
    <div class="bg-darkSlate-800 min-h-screen w-full">
      <Suspense fallback={<EnhancedChangelogSkeleton />}>
        <div class="bg-darkSlate-800/95 border-darkSlate-600 sticky top-0 z-10 border-b backdrop-blur-sm">
          <div class="flex flex-col gap-4 p-4 md:p-6">
            <div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
              <div>
                <h1 class="text-lightSlate-50 mb-1 text-xl font-bold md:text-2xl">
                  Changelog
                </h1>
                <p class="text-lightSlate-600 text-sm">
                  View version history and updates
                </p>
              </div>
              <Show when={routeData.modpackDetails.data}>
                <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
                  <span class="text-lightSlate-400 text-sm font-medium sm:hidden">
                    Version:
                  </span>
                  <span class="text-lightSlate-400 hidden text-sm font-medium sm:inline">
                    Version:
                  </span>
                  <div class="w-full sm:w-auto sm:min-w-48">
                    <Select
                      value={fileId()?.toString()}
                      options={options()}
                      onChange={(key) => {
                        if (key) setFileId(key)
                      }}
                      itemComponent={(props) => (
                        <SelectItem item={props.item}>
                          {optionLabels()[props.item.rawValue] ||
                            props.item.rawValue}
                        </SelectItem>
                      )}
                    >
                      <SelectTrigger class="bg-darkSlate-700 border-darkSlate-600 hover:border-darkSlate-500 w-full transition-colors">
                        <SelectValue<string>>
                          {(state) =>
                            optionLabels()[state.selectedOption() || ""] ||
                            state.selectedOption() ||
                            ""
                          }
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent />
                    </Select>
                  </div>
                </div>
              </Show>
            </div>
          </div>
        </div>

        <div class="p-4 md:p-6">
          <Switch>
            <Match when={isLoading()}>
              <EnhancedChangelogSkeleton />
            </Match>
            <Match when={!hasContent() && !isLoading()}>
              <EmptyChangelogState />
            </Match>
            <Match when={hasContent() && routeData.isCurseforge}>
              <ChangelogCard
                content={changeLog()!}
                type="html"
                platform="curseforge"
                releaseDate={releaseDate()}
              />
            </Match>
            <Match when={hasContent() && !routeData.isCurseforge}>
              <ChangelogCard
                content={changeLog()!}
                type="text"
                platform="modrinth"
                releaseDate={releaseDate()}
              />
            </Match>
          </Switch>
        </div>
      </Suspense>
    </div>
  )
}

export default Changelog
