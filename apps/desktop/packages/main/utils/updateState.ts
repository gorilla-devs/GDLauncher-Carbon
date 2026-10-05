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
export interface UpdateStateData {
  revision: number
  state: UpdateState
  updateInfo: (UpdateInfo & { downloadUrl: string | null }) | null
  progress: number
  error: { message: string; details: string } | null
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
