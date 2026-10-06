import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
import {
  describeLinkedFile,
  describeLinkedFileInFull
} from "@/utils/linkedFile"

/** "Linked file is for 26.3 · NeoForge, so GDLauncher will look for a
 *  compatible version", from the linked file's game versions. Long release
 *  lists are shortened; the title spells every one out. */
export function LinkedFileMismatchNotice(props: {
  gameVersions: readonly string[]
}) {
  const versions = () => describeLinkedFile(props.gameVersions)
  return (
    <span title={describeLinkedFileInFull(props.gameVersions) || undefined}>
      <Show
        when={versions()}
        fallback={<Trans key="search:_trn_linked_file_mismatch_unknown" />}
      >
        <Trans
          key="search:_trn_linked_file_mismatch"
          options={{
            versions: versions(),
            interpolation: { escapeValue: false }
          }}
        />
      </Show>
    </span>
  )
}
