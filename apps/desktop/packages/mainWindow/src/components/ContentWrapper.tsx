interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="w-full h-full max-h-full flex flex-1 justify-center overflow-auto box-border p-5 pb-0 text-white bg-shade-7">
      <div class="rounded-2xl rounded-b-none h-full w-full box-border bg-shade-8 overflow-auto relative">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
