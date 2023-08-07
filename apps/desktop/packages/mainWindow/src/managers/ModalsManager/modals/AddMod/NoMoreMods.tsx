import { Trans } from "@gd/i18n";

const NoMoreMods = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans key="mods.fetching_no_more_mods" />
        </p>
      </div>
    </div>
  );
};

export default NoMoreMods;
