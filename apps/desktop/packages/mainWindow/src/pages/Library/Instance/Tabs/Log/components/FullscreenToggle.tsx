type Props = {
  isFullScreen: () => boolean;
  setIsFullScreen: (_: boolean) => void;
};

export default function FullscreenToggle(props: Props) {
  return (
    <div
      class="w-6 h-6 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out"
      classList={{
        "i-ri:fullscreen-line": !props.isFullScreen(),
        "i-ri:fullscreen-exit-line": props.isFullScreen()
      }}
      onClick={() => {
        props.setIsFullScreen(!props.isFullScreen());
      }}
    />
  );
}
