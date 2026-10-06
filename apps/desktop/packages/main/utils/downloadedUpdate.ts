import type { UpdateInfo } from "electron-updater"

/** Pinned electron-updater 6.8.3 sets this helper before its outer downloaded
 * event on BaseUpdater and MacUpdater. Checking metadata performs no file IO
 * and does not claim native installation or Squirrel completion. */
export function ownsDownloadedUpdate(
  updater: unknown,
  expected: UpdateInfo
): boolean {
  const helper = (
    updater as {
      downloadedUpdateHelper?: {
        file: string | null
        versionInfo: UpdateInfo | null
        fileInfo: { info: { sha512: string } } | null
        downloadedFileInfo: { sha512: string } | null
      }
    }
  ).downloadedUpdateHelper
  const checksum = helper?.fileInfo?.info.sha512
  return !!(
    helper?.file &&
    helper.versionInfo?.version === expected.version &&
    checksum &&
    helper.downloadedFileInfo?.sha512 === checksum &&
    expected.files.some((file) => file.sha512 === checksum)
  )
}
