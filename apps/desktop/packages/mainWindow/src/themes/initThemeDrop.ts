import { onCleanup } from "solid-js"
import { useModal } from "@/managers/ModalsManager"
import { dropTarget } from "./install"

const carriesFiles = (e: DragEvent): boolean =>
  !!e.dataTransfer && Array.from(e.dataTransfer.types).includes("Files")

// Wiring only (imports JSX modules, so unit tests never load it). Any file
// dropped on the window is taken here, so a drop never navigates the window;
// the first .gdltheme among the dropped files opens the install dialog and
// everything else is ignored, as is any drop while the top dialog can't be
// closed (a running install). Nothing is drawn: the dialog is the feedback.
const initThemeDrop = () => {
  const modals = useModal()

  const onDragOver = (e: DragEvent) => {
    if (!carriesFiles(e)) return
    e.preventDefault()
    e.dataTransfer!.dropEffect = "copy"
  }

  const onDrop = (e: DragEvent) => {
    if (!carriesFiles(e)) return
    e.preventDefault()
    const files = Array.from(e.dataTransfer!.files)
    const index = dropTarget(
      files.map((file) => file.name),
      modals?.isTopModalLocked() ?? false
    )
    if (index < 0) return
    const path = window.themeHost.pathForFile(files[index])
    if (!path) return
    modals?.closeModal("themeInstall")
    modals?.openModal({ name: "themeInstall" }, { path })
  }

  // Capture, so an element that stops the event can't let a drop through.
  window.addEventListener("dragover", onDragOver, true)
  window.addEventListener("drop", onDrop, true)
  onCleanup(() => {
    window.removeEventListener("dragover", onDragOver, true)
    window.removeEventListener("drop", onDrop, true)
  })
}

export default initThemeDrop
