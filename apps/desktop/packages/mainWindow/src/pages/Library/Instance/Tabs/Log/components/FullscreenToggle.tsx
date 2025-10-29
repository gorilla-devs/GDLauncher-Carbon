import { AnimatedIcon } from "@gd/ui"

interface Props {
  isFullScreen: () => boolean
  setIsFullScreen: (_: boolean) => void
}

export default function FullscreenToggle(props: Props) {
  return (
    <AnimatedIcon
      icon={
        props.isFullScreen()
          ? "i-hugeicons:minimize-screen"
          : "i-hugeicons:maximize-screen"
      }
      class="bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out"
      size="h-6 w-6"
      onClick={() => {
        props.setIsFullScreen(!props.isFullScreen())
      }}
    />
  )
}
