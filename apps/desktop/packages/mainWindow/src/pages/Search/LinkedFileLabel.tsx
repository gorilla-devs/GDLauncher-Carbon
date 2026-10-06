import { Match, Show, Switch, createMemo } from "solid-js"
import { Trans } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import {
  describeLinkedFile,
  describeLinkedFileInFull,
  LinkedFile,
  LinkedSearchResult,
  toLinkedFile
} from "@/utils/linkedFile"

// Names come from CurseForge; the label renders as text, so skip HTML escaping.
const noEscape = { escapeValue: false }

/** Names the file a link pinned, instead of its raw numeric ID. The versions
 *  come first so they survive truncation; the tooltip has the full text. */
export function LinkedFileLabel(props: { result: LinkedSearchResult }) {
  // Modrinth version links carry only the version ID; look up its name and
  // game versions for the label (the pin itself is unchanged).
  const modrinthVersion = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.modrinth.getVersion",
      props.result.requestedVersionId ?? ""
    ],
    enabled:
      props.result.platform === "modrinth" &&
      !!props.result.requestedVersionId &&
      !props.result.linkedFile
  }))
  const file = createMemo<LinkedFile | undefined>(() => {
    if (props.result.linkedFile) return props.result.linkedFile
    const id = props.result.requestedVersionId
    const version = modrinthVersion.data
    if (
      props.result.platform !== "modrinth" ||
      !id ||
      version?.id !== id ||
      version.project_id !== props.result.id
    )
      return undefined
    return toLinkedFile(id, {
      displayName: version.name || version.version_number,
      gameVersions: [
        ...(version.game_versions ?? []),
        ...(version.loaders ?? [])
      ]
    })
  })
  const versions = () => describeLinkedFile(file()?.gameVersions ?? [])
  const fullText = () => {
    const linked = file()
    if (!linked) return props.result.requestedVersionId ?? ""
    const all = describeLinkedFileInFull(linked.gameVersions)
    return all ? `${linked.displayName} (${all})` : linked.displayName
  }
  return (
    <Switch>
      <Match when={props.result.linkedFileUnavailable}>
        <span
          class="block truncate text-left text-xs text-yellow-400"
          data-testid="linked-file-unavailable"
        >
          <Trans key="search:_trn_linked_file_unavailable" />
        </span>
      </Match>
      <Match when={props.result.requestedVersionId}>
        <span
          class="text-lightSlate-300 block truncate text-left text-xs"
          title={fullText()}
          data-testid="linked-file-label"
        >
          <Show
            when={file()}
            fallback={<Trans key="search:_trn_linked_version" />}
          >
            {(linked) => (
              <Show
                when={versions()}
                fallback={
                  <Trans
                    key="search:_trn_linked_file_name"
                    options={{
                      name: linked().displayName,
                      interpolation: noEscape
                    }}
                  />
                }
              >
                <Trans
                  key="search:_trn_linked_file"
                  options={{
                    name: linked().displayName,
                    versions: versions(),
                    interpolation: noEscape
                  }}
                />
              </Show>
            )}
          </Show>
        </span>
      </Match>
    </Switch>
  )
}
