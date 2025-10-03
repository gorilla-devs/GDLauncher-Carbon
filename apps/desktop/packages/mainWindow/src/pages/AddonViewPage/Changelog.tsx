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
import { Dropdown } from "@gd/ui"
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
    <div class="bg-darkSlate-700 rounded-xl border border-darkSlate-600 shadow-lg overflow-hidden">
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-4 bg-darkSlate-750 border-b border-darkSlate-600">
        <div class="flex items-center gap-3">
          <div class="w-1 h-6 bg-blue-500 rounded-full flex-shrink-0"></div>
          <div>
            <h3 class="font-semibold text-lightSlate-50 text-sm sm:text-base">
              Release Notes
            </h3>
            <Show
              when={props.releaseDate}
              fallback={
                <p class="text-xs text-lightSlate-600">
                  No release date available
                </p>
              }
            >
              <div class="flex items-center gap-2 text-xs text-lightSlate-600">
                <div class="i-ri:calendar-line flex-shrink-0"></div>
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
            <div class="flex items-center gap-1.5 text-orange-500 bg-orange-500/10 px-2 py-1 rounded-full text-xs">
              <div class="i-simple-icons:curseforge w-3 h-3 flex-shrink-0"></div>
              <span class="font-medium">CurseForge</span>
            </div>
          </Show>
          <Show when={props.platform === "modrinth"}>
            <div class="flex items-center gap-1.5 text-green-500 bg-green-500/10 px-2 py-1 rounded-full text-xs">
              <div class="i-simple-icons:modrinth w-3 h-3 flex-shrink-0"></div>
              <span class="font-medium">Modrinth</span>
            </div>
          </Show>
        </div>
      </div>

      <div class="p-4 sm:p-6">
        <Show when={props.type === "html"}>
          <div
            class="prose prose-invert max-w-none prose-sm sm:prose-base
                   prose-headings:text-lightSlate-100 prose-headings:font-semibold
                   prose-p:text-lightSlate-300 prose-p:leading-relaxed prose-p:mb-4
                   prose-a:text-blue-400 prose-a:no-underline hover:prose-a:text-blue-300
                   prose-strong:text-lightSlate-200 prose-strong:font-semibold
                   prose-ul:text-lightSlate-300 prose-ol:text-lightSlate-300
                   prose-li:my-1 prose-li:leading-relaxed
                   prose-code:text-pink-400 prose-code:bg-darkSlate-600 prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded
                   prose-pre:bg-darkSlate-600 prose-pre:border prose-pre:border-darkSlate-500 prose-pre:rounded-lg prose-pre:overflow-x-auto
                   prose-blockquote:border-l-blue-500 prose-blockquote:bg-darkSlate-600 prose-blockquote:p-4 prose-blockquote:rounded-r-lg
                   prose-hr:border-darkSlate-600
                   prose-table:text-sm prose-table:border-collapse prose-th:border prose-th:border-darkSlate-500 prose-th:bg-darkSlate-600 prose-th:p-2
                   prose-td:border prose-td:border-darkSlate-500 prose-td:p-2"
            innerHTML={props.content}
          />
        </Show>
        <Show when={props.type === "text"}>
          <pre class="whitespace-pre-wrap text-lightSlate-300 leading-relaxed font-mono text-xs sm:text-sm bg-darkSlate-600 p-3 sm:p-4 rounded-lg border border-darkSlate-500 overflow-x-auto">
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
      <div class="bg-darkSlate-700 rounded-xl border border-darkSlate-600 overflow-hidden">
        <div class="flex items-center justify-between p-4 bg-darkSlate-750 border-b border-darkSlate-600">
          <div class="flex items-center gap-3">
            <div class="w-1 h-6 bg-darkSlate-500 rounded-full animate-pulse"></div>
            <div>
              <div class="h-5 w-24 bg-darkSlate-500 rounded animate-pulse mb-1"></div>
              <div class="h-3 w-32 bg-darkSlate-500 rounded animate-pulse"></div>
            </div>
          </div>
          <div class="h-6 w-20 bg-darkSlate-500 rounded-full animate-pulse"></div>
        </div>
        <div class="p-6 space-y-4">
          <div class="h-4 w-3/4 bg-darkSlate-500 rounded animate-pulse"></div>
          <div class="h-4 w-full bg-darkSlate-500 rounded animate-pulse"></div>
          <div class="h-4 w-2/3 bg-darkSlate-500 rounded animate-pulse"></div>
          <div class="h-4 w-5/6 bg-darkSlate-500 rounded animate-pulse"></div>
          <div class="h-4 w-1/2 bg-darkSlate-500 rounded animate-pulse"></div>
          <div class="h-4 w-4/5 bg-darkSlate-500 rounded animate-pulse"></div>
        </div>
      </div>
    </div>
  )
}

const EmptyChangelogState = () => {
  return (
    <div class="flex flex-col items-center justify-center py-12 sm:py-16 px-4 sm:px-6 text-center">
      <div class="w-16 h-16 sm:w-24 sm:h-24 bg-darkSlate-600 rounded-full flex items-center justify-center mb-6">
        <div class="i-ri:file-text-line text-2xl sm:text-3xl text-lightSlate-500"></div>
      </div>
      <h3 class="text-lg sm:text-xl font-semibold text-lightSlate-200 mb-2">
        No changelog available
      </h3>
      <p class="text-lightSlate-600 max-w-md leading-relaxed text-sm sm:text-base">
        This version doesn't have any changelog information available. Try
        selecting a different version or check back later.
      </p>
      <div class="mt-6 sm:mt-8 p-3 sm:p-4 bg-darkSlate-700 rounded-lg border border-darkSlate-600 text-left max-w-md w-full">
        <div class="flex items-center gap-2 text-lightSlate-400 text-sm mb-2">
          <div class="i-ri:information-line flex-shrink-0"></div>
          <span class="font-medium">Tip</span>
        </div>
        <p class="text-xs sm:text-sm text-lightSlate-600 leading-relaxed">
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

  const [options, setOptions] = createSignal<{ key: string; label: string }[]>(
    []
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

        setOptions(
          routeData.modrinthProjectVersions.data.map((file) => ({
            key: file.id,
            label: file.version_number
          }))
        )
      }
    } else {
      const sortedVersions = sortArrayByGameVersion(
        routeData.modpackDetails.data?.data.latestFilesIndexes
      )
      setChangelog(undefined)
      setReleaseDate(undefined)
      setIsLoadingChangelog(false)

      setOptions(
        (sortedVersions as CFFEFileIndex[]).map((file) => ({
          key: file.fileId.toString(),
          label: file.filename
        }))
      )
    }
  })

  createEffect((prev) => {
    const modpackId = parseInt(params.id, 10)
    const currentFileId = fileId()
    const currentLastFile = lastFile()
    const isCurseforge = routeData.isCurseforge

    const current = { modpackId, currentFileId, currentLastFile, isCurseforge }

    if (isCurseforge) {
      if (
        currentFileId !== undefined ||
        (currentLastFile && (currentLastFile as CFFEFile).id !== undefined)
      ) {
        setIsLoadingChangelog(true)
        setChangelog(undefined)
        setReleaseDate(undefined)

        const targetFileId =
          parseInt(currentFileId as string, 10) || (currentLastFile as CFFEFile).id

        rspcContext.client.query([
          "modplatforms.curseforge.getModFileChangelog",
          {
            modId: modpackId,
            fileId: targetFileId
          }
        ]).then((changelogQuery) => {
          // Check if parameters haven't changed during async operation
          if (fileId() === currentFileId && routeData.isCurseforge === isCurseforge) {
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
        }).catch((e) => {
          console.error(e)
          if (fileId() === currentFileId && routeData.isCurseforge === isCurseforge) {
            setChangelog(undefined)
            setReleaseDate(undefined)
            setIsLoadingChangelog(false)
          }
        })
      }
    }

    return current
  }, { modpackId: 0, currentFileId: undefined, currentLastFile: undefined, isCurseforge: false })

  createEffect((prev) => {
    const currentFileId = fileId()
    const isCurseforge = routeData.isCurseforge

    const current = { currentFileId, isCurseforge }

    if (!isCurseforge) {
      if (currentFileId !== undefined) {
        setIsLoadingChangelog(true)
        setChangelog(undefined)
        setReleaseDate(undefined)

        rspcContext.client.query([
          "modplatforms.modrinth.getVersion",
          currentFileId as string
        ]).then((changelogQuery) => {
          // Check if parameters haven't changed during async operation
          if (fileId() === currentFileId && !routeData.isCurseforge) {
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
        }).catch((err) => {
          console.error(err)
          if (fileId() === currentFileId && !routeData.isCurseforge) {
            setChangelog(undefined)
            setReleaseDate(undefined)
            setIsLoadingChangelog(false)
          }
        })
      }
    }

    return current
  }, { currentFileId: undefined, isCurseforge: false })

  const isLoading = createMemo(() => isLoadingChangelog())

  const hasContent = createMemo(() => changeLog() && changeLog()?.trim() !== "")

  return (
    <div class="w-full min-h-screen bg-darkSlate-800">
      <Suspense fallback={<EnhancedChangelogSkeleton />}>
        <div class="sticky top-0 z-10 bg-darkSlate-800/95 backdrop-blur-sm border-b border-darkSlate-600">
          <div class="flex flex-col gap-4 p-4 md:p-6">
            <div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
              <div>
                <h1 class="text-xl md:text-2xl font-bold text-lightSlate-50 mb-1">
                  Changelog
                </h1>
                <p class="text-sm text-lightSlate-600">
                  View version history and updates
                </p>
              </div>
              <Show when={routeData.modpackDetails.data}>
                <div class="flex flex-col sm:flex-row sm:items-center gap-3">
                  <span class="text-sm font-medium text-lightSlate-400 sm:hidden">
                    Version:
                  </span>
                  <span class="hidden sm:inline text-sm font-medium text-lightSlate-400">
                    Version:
                  </span>
                  <div class="w-full sm:min-w-48 sm:w-auto">
                    <Dropdown
                      options={options()}
                      onChange={(selectedOption) => {
                        setFileId(selectedOption.key)
                      }}
                      class="w-full bg-darkSlate-700 border-darkSlate-600 hover:border-darkSlate-500 transition-colors"
                    />
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
