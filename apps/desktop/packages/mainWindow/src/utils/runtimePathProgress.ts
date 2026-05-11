import { createSignal } from "solid-js"

interface Progress {
  action: "scan" | "copy" | "remove"
  currentName: string
  current: number
  total: number
}

export const [RTprogress, RTsetProgress] = createSignal<Progress | undefined>()

window.changeRuntimePathProgress((_, _progress: Progress) => {
  RTsetProgress(_progress)
})
