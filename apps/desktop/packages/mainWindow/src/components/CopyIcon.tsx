import { toast } from "@gd/ui"
import { createSignal } from "solid-js"

interface Props {
  text: string | null | undefined | number
}

const CopyIcon = (props: Props) => {
  const [clicked, setClicked] = createSignal(false)

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
        setTimeout(() => {
          setClicked(false)
        }, 600)
      }}
    />
  )
}

export default CopyIcon
