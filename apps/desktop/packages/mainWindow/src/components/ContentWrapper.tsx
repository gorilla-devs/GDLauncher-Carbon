interface Props {
  children: any;
  zeroPadding?: boolean;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="flex justify-center text-lightSlate-50 w-full flex-1 box-border p-4 bg-darkSlate-700 overflow-auto max-h-full min-h-full pb-0 h-content">
      <div
        class="w-full flex-1 box-border overflow-auto flex bg-darkSlate-800 relative flex-col h-auto rounded-2xl rounded-b-none"
        classList={{
          "px-6": !props.zeroPadding
        }}
        style={
          {
            // "scrollbar-gutter": "stable"
          }
        }
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
