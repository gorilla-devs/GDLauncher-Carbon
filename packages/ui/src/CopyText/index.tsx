import { Component, createSignal, Show, splitProps, JSX } from "solid-js"
import { Button } from "../Button"
import { cn } from "../util"

type Size = "small" | "medium" | "large"

interface CopyTextProps extends Omit<
  JSX.HTMLAttributes<HTMLDivElement>,
  "onCopy"
> {
  value: string
  // Written to the clipboard instead of `value` when provided. Use this when
  // the visible text is a shortened/placeholder form (e.g. "https://.../abc")
  // and the full string should be copied.
  copyValue?: string
  size?: Size
  onCopy?: (value: string) => void
}

const sizeStyles: Record<
  Size,
  { container: string; text: string; overlay: string }
> = {
  small: {
    container: "px-3 py-2 gap-2",
    text: "text-sm",
    overlay: "text-sm"
  },
  medium: {
    container: "px-4 py-2.5 gap-2",
    text: "text-base",
    overlay: "text-base"
  },
  large: {
    container: "px-6 py-3 gap-3",
    text: "text-2xl font-bold tracking-wider",
    overlay: "text-base"
  }
}

type CopyState = "idle" | "showing" | "leaving"

export const CopyText: Component<CopyTextProps> = (props) => {
  const [local, others] = splitProps(props, [
    "value",
    "copyValue",
    "size",
    "class",
    "onCopy"
  ])
  const [copyState, setCopyState] = createSignal<CopyState>("idle")

  const size = () => local.size ?? "small"
  const styles = () => sizeStyles[size()]

  const handleCopy = async () => {
    if (copyState() !== "idle") return

    const toCopy = local.copyValue ?? local.value
    try {
      await navigator.clipboard.writeText(toCopy)
      local.onCopy?.(toCopy)
      setCopyState("showing")

      // After 2s, start exit animation
      setTimeout(() => {
        setCopyState("leaving")
        // After exit animation completes (300ms), reset to idle
        setTimeout(() => setCopyState("idle"), 300)
      }, 2000)
    } catch (err) {
      console.error("Failed to copy:", err)
    }
  }

  return (
    <div
      class={cn(
        "bg-darkSlate-700 relative flex items-center rounded overflow-hidden",
        styles().container,
        local.class
      )}
      {...others}
    >
      {/* Text content */}
      <span
        class={cn(
          "text-lightSlate-400 select-all flex-1 min-w-0 truncate",
          styles().text
        )}
      >
        {local.value}
      </span>

      {/* Copy button */}
      <Button
        type="text"
        size="small"
        onClick={handleCopy}
        classList={{
          "z-10": true,
          "!bg-darkSlate-600": true,
          "hover:!bg-darkSlate-500": true,
          "!text-lightSlate-500": true,
          "hover:!text-lightSlate-50": true
        }}
      >
        <div class="i-hugeicons:copy-01 h-4 w-4" />
        Copy
      </Button>

      {/* Copied overlay - full container coverage */}
      <Show when={copyState() !== "idle"}>
        {/* Blur layer */}
        <div
          class={cn(
            "absolute inset-0 z-20",
            copyState() === "showing" && "copy-text-blur-in",
            copyState() === "leaving" && "copy-text-blur-out"
          )}
        />
        {/* Solid background layer */}
        <div
          class={cn(
            "absolute inset-0 z-20 bg-darkSlate-600",
            copyState() === "showing" && "copy-text-solid-in",
            copyState() === "leaving" && "copy-text-solid-out"
          )}
        />
        {/* Progress bar that fills left to right */}
        <Show when={copyState() === "showing"}>
          <div class="absolute inset-0 z-20 bg-darkSlate-500 origin-left copy-text-fill" />
        </Show>
        {/* Copied text - centered in full container */}
        <div
          class={cn(
            "absolute inset-0 z-20 flex items-center justify-center",
            copyState() === "showing" && "copy-text-drop-in",
            copyState() === "leaving" && "copy-text-drop-out"
          )}
        >
          <span
            class={cn(
              "text-lightSlate-200 font-medium flex items-center gap-1",
              styles().overlay
            )}
          >
            <div class="i-hugeicons:checkmark-circle-02 h-4 w-4" />
            Copied
          </span>
        </div>
      </Show>
    </div>
  )
}
