interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="w-full h-full flex flex-1 justify-center box-border p-5 text-white max-h-full overflow-auto pb-0 bg-shade-7">
      <div class="h-full w-full box-border bg-shade-8 overflow-auto relative rounded-2xl rounded-b-none">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
