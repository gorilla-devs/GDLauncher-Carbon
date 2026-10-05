import type { AppUpdater, UpdateInfo } from "electron-updater"
import { createClient } from "electron-updater/out/providerFactory"
import type { Provider } from "electron-updater/out/providers/Provider"
import { gt } from "semver"
import { selectUpdateArtifact, type Installation } from "./updateArtifact"

/** Isolated electron-updater 6.8.3 adaptation. Protected fields are not a public
 * API: doCheckForUpdates assigns info/provider before update-available. Reading
 * resolveFiles is side-effect-free. Null/manual checks initialize only the same
 * active client (override wins), then call its metadata-only getLatestVersion. */
interface PinnedUpdater {
  clientPromise: Promise<Provider<UpdateInfo>> | null
  updateInfoAndProvider: {
    info: UpdateInfo
    provider: Provider<UpdateInfo>
  } | null
  configOnDisk: { value: Promise<Parameters<typeof createClient>[0]> }
  createProviderRuntimeOptions(): Parameters<typeof createClient>[2]
}
export function activeUpdateMetadata(
  updater: AppUpdater,
  installation: Installation,
  currentVersion: string
) {
  const pinned = updater as unknown as PinnedUpdater
  return {
    artifact(info: UpdateInfo) {
      const active = pinned.updateInfoAndProvider
      if (active?.info !== info) return null
      return selectUpdateArtifact(
        active.provider.resolveFiles(info),
        installation
      )
    },
    /** The channel's latest version, compared with this build before any
     *  download is looked for: an installation that is up to date has no
     *  update, even when the channel publishes nothing for its format. A newer
     *  version without a matching download comes with `downloadUrl: null`. */
    async manual(): Promise<
      { noUpdate: true } | (UpdateInfo & { downloadUrl: string | null })
    > {
      if (!pinned.clientPromise)
        pinned.clientPromise = pinned.configOnDisk.value.then((config) =>
          createClient(config, updater, pinned.createProviderRuntimeOptions())
        )
      const provider = await pinned.clientPromise
      const info = await provider.getLatestVersion()
      if (
        info.version === currentVersion ||
        (!updater.allowDowngrade && !gt(info.version, currentVersion))
      )
        return { noUpdate: true }
      return {
        ...info,
        downloadUrl: selectUpdateArtifact(
          provider.resolveFiles(info),
          installation
        )
      }
    }
  }
}
