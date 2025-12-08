interface Props {
  isFullScreen: () => boolean
  setIsFullScreen: (_: boolean) => void
}

export default function FullscreenToggle(props: Props) {
  return (
    <div
      class="animate-icons-on-hover cursor-pointer"
      onClick={() => {
        props.setIsFullScreen(!props.isFullScreen())
      }}
    >
      <div
        class={`h-5 w-5 bg-lightSlate-800 transition-colors duration-200 ease-in-out ${
          props.isFullScreen()
            ? "i-hugeicons:minimize-screen"
            : "i-hugeicons:maximize-screen"
        }`}
      />
    </div>
  )
}
