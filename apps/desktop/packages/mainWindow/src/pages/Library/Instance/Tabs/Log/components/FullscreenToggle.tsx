interface Props {
  isFullScreen: () => boolean
  setIsFullScreen: (_: boolean) => void
}

export default function FullscreenToggle(props: Props) {
  return (
    <div
      class={`h-6 w-6 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out ${
        props.isFullScreen()
          ? "i-hugeicons:minimize-screen"
          : "i-hugeicons:maximize-screen"
      }`}
      onClick={() => {
        props.setIsFullScreen(!props.isFullScreen())
      }}
    />
  )
}
