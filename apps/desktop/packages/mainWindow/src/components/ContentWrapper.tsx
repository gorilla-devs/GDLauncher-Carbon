interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="flex flex-1 justify-center text-white w-full h-full box-border max-h-full bg-darkSlate-700 p-5 overflow-auto pb-0">
      <div class="h-full w-full box-border bg-darkSlate-800 overflow-auto relative rounded-2xl rounded-b-none">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
