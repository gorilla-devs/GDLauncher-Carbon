import { useTransContext } from "@gd/i18n"
import type {
  FEJavaEntry,
  FEJavaGroup,
  FEManagedJavaInstall
} from "@gd/core_module/bindings"
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  createUniqueId,
  onCleanup
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { labelHints } from "@/utils/java/javaText"
import { groupHeader } from "./javaOverviewView"
import IconButton from "./IconButton"
import JavaRow, { JavaInstallRow } from "./JavaRow"

/** A rule above every group but the first: the groups share one box. */
export const GROUP_CLASS = "border-darkSlate-600 [&:not(:first-child)]:border-t"

interface Props {
  /** Missing for "Other Java versions", the majors no Minecraft uses. */
  group?: FEJavaGroup
  /** Whether the rows' `usedBy` is the whole story yet. */
  usageComplete: boolean
  javas: FEJavaEntry[]
  installs: FEManagedJavaInstall[]
  /** Takes focus when the group goes away with focus inside it, as "Other
   *  Java versions" does when its last Java is deleted. */
  onFocusLost?: () => void
}

const JavaGroup = (props: Props) => {
  const [t] = useTransContext()
  const headingId = createUniqueId()
  let section: HTMLElement | undefined
  let heading: HTMLHeadingElement | undefined

  // Runs before the group's nodes leave the page, so focus is still inside.
  onCleanup(() => {
    if (section?.contains(document.activeElement)) props.onFocusLost?.()
  })

  // The Java whose marker takes focus once the list shows it in use. The
  // clicked button turns into the in-use marker, and the row moves first.
  const [focusAfterRefresh, setFocusAfterRefresh] = createSignal<string>()

  const setDefault = rspc.createMutation(() => ({
    mutationKey: ["java.setMajorDefault"]
  }))

  // Where focus goes when the control that had it goes away.
  const focusHeading = () => heading?.focus()

  const chooseJava = (javaId: string) => {
    const group = props.group
    if (!group) return
    setDefault.mutate(
      { major: group.major, javaId },
      { onSuccess: () => setFocusAfterRefresh(javaId) }
    )
  }

  const chooseAutomatically = (major: number) =>
    setDefault.mutate({ major, javaId: null }, { onSuccess: focusHeading })

  const title = () =>
    props.group
      ? groupHeader(props.group, t)
      : t("java:_trn_other_java_versions")
  const empty = () => props.javas.length === 0 && props.installs.length === 0
  // Found and added Javas show their path, which tells them apart already.
  const hint = (entry: FEJavaEntry) =>
    entry.source === "managed"
      ? labelHints(props.javas).get(entry.id)
      : undefined

  return (
    <section ref={section} aria-labelledby={headingId} class={GROUP_CLASS}>
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <h5
          ref={heading}
          id={headingId}
          tabIndex={-1}
          class="text-lightSlate-50 m-0 rounded text-base font-medium outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
        >
          {title()}
        </h5>
        <Show when={props.group}>
          {(group) => (
            <Switch>
              <Match when={group().userChosen}>
                <IconButton
                  label={t("java:_trn_a11y_let_automatic_pick", {
                    major: group().major
                  })}
                  tooltip={t("java:_trn_let_automatic_pick")}
                  class="text-lightSlate-700 hover:text-lightSlate-50"
                  onClick={() => chooseAutomatically(group().major)}
                >
                  <div class="i-hugeicons:arrow-turn-backward h-5 w-5" />
                </IconButton>
              </Match>
              <Match when={group().downloadsWhenNeeded && empty()}>
                <span class="text-lightSlate-700 text-sm">
                  {t("java:_trn_downloads_when_needed")}
                </span>
              </Match>
            </Switch>
          )}
        </Show>
      </div>
      <Show when={!empty()}>
        <div class="flex flex-col px-2 pb-2">
          <For each={props.javas}>
            {(entry) => (
              <JavaRow
                entry={entry}
                group={props.group}
                usageComplete={props.usageComplete}
                hint={hint(entry)}
                onChoose={() => chooseJava(entry.id)}
                focusMarker={focusAfterRefresh() === entry.id}
                onMarkerFocused={() => setFocusAfterRefresh(undefined)}
                focusHeading={focusHeading}
              />
            )}
          </For>
          <For each={props.installs}>
            {(install) => <JavaInstallRow install={install} />}
          </For>
        </div>
      </Show>
    </section>
  )
}

export default JavaGroup
