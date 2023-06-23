interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="flex flex-1 justify-center text-white w-full box-border h-full max-h-full bg-darkSlate-700 p-5 overflow-auto pb-0">
      <div class="w-full box-border bg-darkSlate-800 overflow-auto relative rounded-2xl rounded-b-none flex flex-col py-5 h-auto px-6">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
