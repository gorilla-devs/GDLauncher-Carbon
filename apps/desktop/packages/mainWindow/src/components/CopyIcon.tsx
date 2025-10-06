import { toast } from "@gd/ui"
import { createSignal, onCleanup } from "solid-js"

interface Props {
  text: string | null | undefined | number
}

const CopyIcon = (props: Props) => {
  const [clicked, setClicked] = createSignal(false)
  let timeoutId: number | undefined

  onCleanup(() => {
    if (timeoutId !== undefined) {
      clearTimeout(timeoutId)
    }
  })

  return (
    <div
      class="hover:text-lightSlate-50 i-ri:clipboard-line transition-transform duration-200 hover:scale-120"
      classList={{
        "animate-scaleBounce": clicked()
      }}
      onClick={() => {
        if (!props.text) return
        navigator.clipboard.writeText(props.text as string)
        toast.success("Copied to clipboard")
        setClicked(true)

        // Clear any existing timeout before setting a new one
        if (timeoutId !== undefined) {
          clearTimeout(timeoutId)
        }

        timeoutId = setTimeout(() => {
          setClicked(false)
          timeoutId = undefined
        }, 600)
      }}
    />
  )
}

export default CopyIcon
