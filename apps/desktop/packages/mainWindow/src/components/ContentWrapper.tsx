import { useLocation } from "@solidjs/router";

interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  const location = useLocation();

  // it match /library/2 but not /library
  const libraryPathRegex = /\/library\/(\d+)(\/.*)?/;
  const modpackPathRegex = /\/modpacks\/(\d+)(\/.*)?/;
  const isInstanceDetails = () => libraryPathRegex.test(location.pathname);
  const isModpackDetails = () => modpackPathRegex.test(location.pathname);

  return (
    <div class="flex flex-1 justify-center text-white w-full box-border h-full max-h-full bg-darkSlate-700 p-5 overflow-auto pb-0">
      <div
        class="w-full box-border overflow-auto flex bg-darkSlate-800 relative flex-col h-auto rounded-2xl rounded-b-none"
        classList={{
          "scrollbar-gutter p-6 pr-3":
            !isInstanceDetails() && !isModpackDetails(),
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
