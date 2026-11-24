import { DecorationSlot } from "@/themes/DecorationSlot"

interface Props {
  children: any
  zeroPadding?: boolean
}
const ContentWrapper = (props: Props) => {
  return (
    <div
      data-gdl="panel-frame"
      style={{
        // Documented for themes: the panel box inside this frame, and its radius.
        "--gdl-panel-inset": "1rem 1rem 0 1rem",
        "--gdl-panel-radius":
          "calc(1rem * var(--radius-scale, 1)) calc(1rem * var(--radius-scale, 1)) 0 0"
      }}
      class="text-lightSlate-50 bg-darkSlate-700 relative h-content box-border flex max-h-full min-h-full w-full flex-1 justify-center overflow-hidden p-4 pb-0"
    >
      <div
        data-gdl="panel"
        id="gdl-content-wrapper"
        class="bg-darkSlate-800 relative isolate box-border flex h-auto w-full flex-1 flex-col overflow-auto rounded-2xl rounded-b-none"
        classList={{
          "p-6": !props.zeroPadding
        }}
        style={{
          "scrollbar-gutter": "stable"
        }}
      >
        {props.children}
      </div>
      <DecorationSlot
        anchor="panel"
        class="!left-4 !right-4 !top-4 !bottom-0"
      />
    </div>
  )
}

export default ContentWrapper
