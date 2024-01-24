interface Props {
  children: any;
}
const ModpackBrowserWrapper = (props: Props) => {
  return (
    <div class="flex flex-1 justify-center text-white w-full box-border h-full bg-darkSlate-700 max-h-full pb-0 p-5">
      <div class="h-full w-full box-border bg-darkSlate-800 overflow-hidden relative rounded-2xl rounded-b-none">
        {props.children}
      </div>
    </div>
  );
};

export default ModpackBrowserWrapper;
