interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  return (
    <div class="w-full h-full max-h-full flex flex-1 justify-center overflow-auto box-border p-5 text-white bg-black-semiblack">
      <div class="rounded-2xl h-fit w-full box-border bg-black-black p-6">
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
