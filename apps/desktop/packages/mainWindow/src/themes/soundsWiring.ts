import { createEffect, createMemo, onCleanup, untrack } from "solid-js"
import { unwrap } from "solid-js/store"
import type { FETask, FEThemeSounds } from "@gd/core_module/bindings"
import type { MotionState } from "./motion"
import {
  downloadsFinished,
  hasSounds,
  isPrimaryButton,
  launchesStarted,
  sameSounds,
  toastSound,
  watchToasts,
  type SoundBank
} from "./sounds"

export const TOAST_REGION = '[data-gdl~="toast-region"]'

/** The part of the `vtask.getTasks` query the wiring reads. */
export interface TaskList {
  data: FETask[] | undefined
  dataUpdatedAt: number
}

/** The part of `document` the wiring uses. */
export interface SoundsDocument {
  addEventListener(
    type: "click",
    listener: (e: MouseEvent) => void,
    capture: boolean
  ): void
  removeEventListener(
    type: "click",
    listener: (e: MouseEvent) => void,
    capture: boolean
  ): void
  querySelector(selector: string): Element | null
}

export interface SoundsWiringDeps {
  /** `themes.getPreferences`' data. */
  preferences(): { sounds: boolean } | undefined
  /** The saved theme's sounds: committedTheme(), never a hover preview. */
  sounds(): FEThemeSounds | null | undefined
  motion(): MotionState
  createBank(sounds: FEThemeSounds, motion: MotionState): SoundBank
  /** Makes the `vtask.getTasks` query, owned by the caller's scope. */
  createTaskList(): TaskList
  /** Wall-clock ms, the clock of `dataUpdatedAt`. */
  now(): number
  document: SoundsDocument
}

/**
 * Nobody who leaves sounds off pays for them: the players, the click
 * listener, the toast watcher and the task query exist only while sounds are
 * switched on and the saved theme has a file for them, and they all go when
 * either stops. Call it under an owner (a component or a root).
 */
export function wireSounds(deps: SoundsWiringDeps): void {
  // Memos, so only a change of these rebuilds the players: every preferences
  // write (another switch, a tile size pick) is a new `data` object, and the
  // leave after a hover preview commits an equal theme as a new object.
  const enabled = createMemo(() => deps.preferences()?.sounds === true)
  const sounds = createMemo(() => deps.sounds() ?? null, null, {
    equals: sameSounds
  })
  let warnedRegion = false

  // An effect, so the toast region is looked up after mount.
  createEffect(() => {
    const s = sounds()
    if (!s || !enabled() || !hasSounds(s)) return
    untrack(() => {
      const bank = deps.createBank(s, deps.motion())
      onCleanup(() => bank.dispose())
      createEffect(() => bank.setMotion(deps.motion()))

      if (s.click) {
        const onClick = (e: MouseEvent) => {
          if (isPrimaryButton(e.target)) bank.play("click")
        }
        deps.document.addEventListener("click", onClick, true)
        onCleanup(() =>
          deps.document.removeEventListener("click", onClick, true)
        )
      }

      if (s.notification || s.error) {
        const region = deps.document.querySelector(TOAST_REGION)
        if (region) {
          onCleanup(
            watchToasts(region, (type) => {
              const event = toastSound(type)
              if (event) bank.play(event)
            })
          )
        } else if (!warnedRegion) {
          warnedRegion = true
          console.warn("Theme sounds: no toast region, toasts stay silent")
        }
      }

      if (s.launch || s.downloadComplete) {
        // Each watcher starts its own baseline: the first list fetched after
        // it exists. A list cached before then may be minutes old, and a
        // download that finished meanwhile must not play.
        const since = deps.now()
        const tasks = deps.createTaskList()
        let prev: FETask[] | undefined
        createEffect(() => {
          const data = tasks.data
          if (data === undefined || tasks.dataUpdatedAt < since) return
          const next = unwrap(data)
          if (launchesStarted(prev, next)) bank.play("launch")
          if (downloadsFinished(prev, next)) bank.play("downloadComplete")
          prev = next
        })
      }
    })
  })
}
