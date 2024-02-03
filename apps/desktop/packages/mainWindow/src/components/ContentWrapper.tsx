import { useLocation } from "@solidjs/router";

interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  const location = useLocation();

  const dynamicPathRegex = /(\/library|modpacks|mods)\/(\d+)(\/.*)?/;

  return (
    <div class="flex justify-center text-white w-full flex-1 box-border p-4 flex-1 h-full bg-darkSlate-700 overflow-auto max-h-full pb-0">
      <div
        class="w-full flex-1 box-border overflow-auto flex bg-darkSlate-800 relative flex-col h-auto rounded-2xl rounded-b-none"
        classList={{
          "scrollbar-gutter py-6 pl-6 pr-3": !dynamicPathRegex.test(
            location.pathname
          )
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
