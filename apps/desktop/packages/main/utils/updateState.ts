import type { UpdateInfo } from "electron-updater"
export type UpdateState =
  | "idle"
  | "checking"
  | "downloading"
  | "downloaded"
  | "error"
  | "no-update"
  | "unsupported"
  | "manual-update"
/** Why an update step failed. The renderer shows each code as translated text;
 *  `message` is English for logs and the untranslated loading screen. */
export type UpdateErrorCode =
  | "prepare-failed"
  | "download-unavailable"
  | "update-changed"
  | "update-failed"
  | "check-failed"
  | "no-manual-download"
  | "manual-check-failed"
export interface UpdateStateData {
  revision: number
  state: UpdateState
  updateInfo: (UpdateInfo & { downloadUrl: string | null }) | null
  progress: number
  error: { message: string; details: string; code?: UpdateErrorCode } | null
  /** Positive completed artifact ownership, independent of the current check phase. */
  readyUpdate?: { id: number; version: string } | null
}
export interface CheckForUpdatesResult {
  status:
    | "checking"
    | "busy"
    | "snapshot-version"
    | "unsupported"
    | "manual-update"
    | "no-update"
    | "error"
  currentState: UpdateStateData
}
