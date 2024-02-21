interface Props {
  children: any;
  zeroPadding?: boolean;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="flex justify-center text-white w-full flex-1 box-border p-4 bg-darkSlate-700 overflow-auto max-h-full pb-0 h-content">
      <div
        class="w-full flex-1 box-border overflow-auto flex bg-darkSlate-800 relative flex-col h-auto rounded-2xl rounded-b-none scrollbar-gutter pb-10 pr-3"
        classList={{
          "pl-6": !props.zeroPadding
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
