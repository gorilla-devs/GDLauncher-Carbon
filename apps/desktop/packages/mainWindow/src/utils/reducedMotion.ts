import { createEffect, onCleanup, type Accessor } from "solid-js"

// Kept separate from main's startup so the physical preference effect can be
// exercised without launching the application.
export function applyReducedMotion(reducedMotion: Accessor<boolean>) {
  createEffect(() => {
    document.body.classList.toggle("potato-pc", reducedMotion())
  })
  onCleanup(() => document.body.classList.remove("potato-pc"))
}
