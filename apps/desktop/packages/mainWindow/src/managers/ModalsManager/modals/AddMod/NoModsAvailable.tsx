import { Trans } from "@gd/i18n";
import skull from "/assets/images/icons/skull.png";

const NoModsAvailable = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-100">
      <div class="flex justify-center items-center flex-col text-center">
        <img src={skull} class="w-16 h-16" />

        <p class="text-darkSlate-50 max-w-100">
          <Trans key="mods.fetching_no_mods_available" />
        </p>
      </div>
    </div>
  );
};

export default NoModsAvailable;
