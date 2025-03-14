interface Props {
  children: any
  zeroPadding?: boolean
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="text-lightSlate-50 bg-darkSlate-700 h-content box-border flex max-h-full min-h-full w-full flex-1 justify-center overflow-hidden p-4 pb-0">
      <div
        id="gdl-content-wrapper"
        class="bg-darkSlate-800 relative box-border flex h-auto w-full flex-1 flex-col overflow-auto rounded-2xl rounded-b-none"
        classList={{
          "p-6": !props.zeroPadding
        }}
        style={{
          "scrollbar-gutter": "stable"
        }}
      >
        {props.children}
      </div>
    </div>
  )
}

export default ContentWrapper
