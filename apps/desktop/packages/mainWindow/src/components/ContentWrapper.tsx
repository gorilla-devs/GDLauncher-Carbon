import { NAVBAR_ROUTES } from "@/constants";
import { useLocation } from "@solidjs/router";

interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  const location = useLocation();

  const labels = NAVBAR_ROUTES.map((route) => route.label).join("|");
  // this match /library/2 but not /library (or modpacks/2 / mods/2 ...)
  const dynamicPathRegex = new RegExp(`(\\/${labels})\\/(\\d+)(\\/.*)?`, "g");

  const isNestedRoute = () => dynamicPathRegex.test(location.pathname);

  return (
    <div class="flex flex-1 justify-center text-white w-full box-border h-full bg-darkSlate-700 p-5 overflow-auto max-h-full pb-0">
      <div
        class="w-full box-border overflow-auto flex bg-darkSlate-800 relative flex-col rounded-2xl h-auto rounded-b-none"
        classList={{
          "scrollbar-gutter p-6 pr-3": !isNestedRoute(),
        }}
      >
        {props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
