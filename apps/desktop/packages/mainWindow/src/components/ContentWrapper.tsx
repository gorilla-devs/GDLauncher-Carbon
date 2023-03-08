interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="flex justify-center text-white w-full box-border flex-1 h-full max-h-full bg-shade-7 p-5 overflow-auto pb-0">
      <div class="h-full w-full box-border bg-shade-8 overflow-auto relative rounded-2xl rounded-b-none">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
