import { createSignal } from "solid-js"

interface Progress {
  // "flush" writes the finished copy to the drive; "rollback" removes what a
  // move that stopped had copied.
  action: "check" | "scan" | "copy" | "flush" | "remove" | "rollback"
  currentName: string
  current: number
  total: number
}

export const [RTprogress, RTsetProgress] = createSignal<Progress | undefined>()

window.changeRuntimePathProgress((_, _progress: Progress) => {
  RTsetProgress(_progress)
})
