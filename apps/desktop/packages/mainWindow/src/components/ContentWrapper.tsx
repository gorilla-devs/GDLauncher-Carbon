import { useMatch } from "@solidjs/router";

interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  const isSettings = useMatch(() => "/settings/*");

  return (
    <div class="flex flex-1 justify-center text-white w-full box-border h-full max-h-full bg-darkSlate-700 p-5 overflow-auto pb-0">
      <div
        class="w-full box-border bg-darkSlate-800 overflow-auto relative flex flex-col h-auto rounded-2xl rounded-b-none scrollbar-gutter"
        classList={{
          "px-6": !!isSettings(),
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
