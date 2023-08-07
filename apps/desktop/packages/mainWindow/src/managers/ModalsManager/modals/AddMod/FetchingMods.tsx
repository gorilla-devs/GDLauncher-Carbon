import { Trans } from "@gd/i18n";
import { Spinner } from "@gd/ui";

const FetchingMods = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans key="mods.fetching_mods_text" />
        </p>
        <Spinner />
      </div>
    </div>
  );
};

export default FetchingMods;
