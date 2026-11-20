import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Skeleton,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  toast
} from "@gd/ui"
import type {
  FEJavaOverview,
  FEManagedJavaInstall
} from "@gd/core_module/bindings"
import type { UseQueryResult } from "@tanstack/solid-query"
import { For, Match, Show, Switch, createSignal } from "solid-js"
import { useModal } from "@/managers/ModalsManager"
import { TextLink } from "@/components/TextLink"
import type { RemoveUnusedJavasConfirmData } from "@/managers/ModalsManager/modals/Java/RemoveUnusedJavasConfirm"
import { useFormatSize } from "@/utils/java/useFormatSize"
import { rspc, type RSPCError } from "@/utils/rspcClient"
import Title from "../components/Title"
import { addJavaFromFile } from "./addJavaFromFile"
import {
  installRowsFor,
  removedUnusedMessage,
  unusedJavas
} from "./javaOverviewView"
import JavaGroup, { GROUP_CLASS } from "./JavaGroup"

/** One box for every group, as one list. */
export const CONTAINER_CLASS =
  "border-darkSlate-600 flex flex-col rounded-xl border border-solid"

interface Props {
  overview: UseQueryResult<FEJavaOverview, RSPCError>
  installs: FEManagedJavaInstall[]
}

const YourJavas = (props: Props) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const size = useFormatSize()
  let addButton: HTMLButtonElement | undefined
  let removeUnusedLink: HTMLButtonElement | undefined
  const [adding, setAdding] = createSignal(false)

  const removeUnused = rspc.createMutation(() => ({
    mutationKey: ["java.removeUnusedJavas"]
  }))

  /** Installs of majors no Minecraft version uses, not listed yet. */
  const otherInstalls = (overview: FEJavaOverview) => {
    const grouped = new Set(overview.groups.map((g) => g.major))
    return props.installs.filter(
      (i) =>
        !grouped.has(i.major) &&
        installRowsFor(i.major, [i], overview.otherJavas).length > 0
    )
  }

  const confirmRemoveUnused = (overview: FEJavaOverview) => {
    const data: RemoveUnusedJavasConfirmData = {
      javas: unusedJavas(overview).map((e) => ({
        id: e.id,
        name: `${e.vendorName} ${e.version}`,
        sizeBytes: e.sizeBytes
      })),
      totalBytes: overview.unusedSizeBytes,
      onConfirm: async () => {
        const removed = await removeUnused.mutateAsync(undefined)
        toast.success(removedUnusedMessage(removed, t))
      },
      // Once they're gone, so is the link.
      onClosed: (removed) => (removed ? addButton : removeUnusedLink)?.focus()
    }
    modalsContext?.openModal({ name: "removeUnusedJavasConfirm" }, data)
  }

  const addFromFile = async () => {
    if (adding()) return
    setAdding(true)
    try {
      const added = await addJavaFromFile(t)
      if (added) {
        toast.success(
          t("java:_trn_added_java", {
            vendor: added.vendorName,
            version: added.version
          })
        )
      }
    } finally {
      setAdding(false)
    }
  }

  return (
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between gap-4">
        <Title>
          <Trans key="java:_trn_your_javas" />
        </Title>
        <div class="flex items-center gap-4">
          <Show
            when={
              props.overview.data?.unusedJavaIds.length
                ? props.overview.data
                : undefined
            }
          >
            {(overview) => (
              <TextLink
                ref={(el) => (removeUnusedLink = el)}
                tone="muted"
                class="whitespace-nowrap"
                onClick={() => confirmRemoveUnused(overview())}
              >
                {overview().unusedSizeBytes > 0
                  ? t("java:_trn_remove_unused_size", {
                      size: size(overview().unusedSizeBytes)
                    })
                  : t("java:_trn_remove_unused")}
              </TextLink>
            )}
          </Show>
          <DropdownMenu placement="bottom-end">
            <Tooltip>
              <DropdownMenuTrigger
                as={TooltipTrigger}
                ref={addButton}
                class="b-0 rounded-md bg-transparent p-0 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
                aria-label={t("java:_trn_add_java")}
              >
                <Button as="div" type="secondary" size="small">
                  <div class="i-hugeicons:add-01 text-xl" />
                </Button>
              </DropdownMenuTrigger>
              <TooltipContent>{t("java:_trn_add_java")}</TooltipContent>
            </Tooltip>
            <DropdownMenuContent>
              <DropdownMenuItem
                onSelect={() =>
                  modalsContext?.openModal({ name: "addManagedJava" })
                }
              >
                <div class="flex items-center gap-2">
                  <i class="i-hugeicons:download-02" />
                  <span>
                    <Trans key="java:_trn_download_java" />
                  </span>
                </div>
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={addFromFile}>
                <div class="flex items-center gap-2">
                  <i class="i-hugeicons:folder-open" />
                  <span>
                    <Trans key="java:_trn_add_from_file" />
                  </span>
                </div>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <Switch
        fallback={
          <div class={CONTAINER_CLASS} aria-busy="true">
            <For each={[0, 1]}>
              {() => (
                <div class={`${GROUP_CLASS} flex flex-col gap-3 p-4`}>
                  <Skeleton class="h-5 w-64" />
                  <Skeleton class="h-4 w-48" />
                </div>
              )}
            </For>
          </div>
        }
      >
        <Match when={props.overview.data}>
          {(overview) => (
            <div class={CONTAINER_CLASS}>
              <For each={overview().groups}>
                {(group) => (
                  <JavaGroup
                    group={group}
                    usageComplete={overview().usageComplete}
                    javas={group.javas}
                    installs={installRowsFor(
                      group.major,
                      props.installs,
                      group.javas
                    )}
                  />
                )}
              </For>
              <Show
                when={
                  overview().otherJavas.length > 0 ||
                  otherInstalls(overview()).length > 0
                }
              >
                <JavaGroup
                  usageComplete={overview().usageComplete}
                  javas={overview().otherJavas}
                  installs={otherInstalls(overview())}
                  onFocusLost={() => addButton?.focus()}
                />
              </Show>
            </div>
          )}
        </Match>
        <Match when={props.overview.isError && !props.overview.isFetching}>
          <div class="flex items-center gap-1.5 text-sm">
            <span class="text-red-300">
              <Trans key="java:_trn_overview_failed" />
            </span>
            <span class="text-darkSlate-400">·</span>
            <TextLink onClick={() => props.overview.refetch()}>
              <Trans key="java:_trn_try_again" />
            </TextLink>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default YourJavas
