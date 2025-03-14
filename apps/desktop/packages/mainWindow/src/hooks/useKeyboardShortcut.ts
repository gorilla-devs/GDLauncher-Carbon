import { onCleanup } from "solid-js"

function useKeyboardShortcut(keys: string[], callback: () => void) {
  const pressedKeys = new Set<string>()
  let callbackInvoked = false // Flag to track if the callback has been invoked

  const handler = (event: KeyboardEvent) => {
    // console.log("keyDownHandler", event.key)
    pressedKeys.add(event.key.toLowerCase())

    const isMatch = keys.every((key) => pressedKeys.has(key.toLowerCase()))

    if (isMatch && !callbackInvoked) {
      event.preventDefault() // Prevent default action if needed
      callback()
      callbackInvoked = true // Set the flag to true after invoking the callback
    }
  }

  const keyUpHandler = (event: KeyboardEvent) => {
    // console.log("keyUpHandler", event.key)
    pressedKeys.delete(event.key.toLowerCase())
    // Reset the callback invoked flag when any key is released
    if (pressedKeys.size === 0) {
      callbackInvoked = false
    }
  }

  const metaKeyUpHandler = (event: KeyboardEvent) => {
    if (event.key === "Meta") {
      // Clear all pressed keys when the Meta key is released
      pressedKeys.clear()
      callbackInvoked = false // Reset the flag when Meta key is released
    }
  }

  window.addEventListener("keydown", handler)
  window.addEventListener("keyup", keyUpHandler)
  window.addEventListener("keyup", metaKeyUpHandler) // Listen for Meta key release

  onCleanup(() => {
    window.removeEventListener("keydown", handler)
    window.removeEventListener("keyup", keyUpHandler)
    window.removeEventListener("keyup", metaKeyUpHandler) // Clean up Meta key listener
  })
}

export default useKeyboardShortcut
