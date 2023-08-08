import { Trans } from "@gd/i18n";
import { Spinner } from "@gd/ui";
import skull from "/assets/images/icons/skull.png";

const NoModpacksAvailable = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 bg-darkSlate-700 rounded-xl h-100 mx-5">
      <div class="flex justify-center items-center flex-col text-center">
        <img src={skull} class="w-16 h-16" />

        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_no_modpacks_available"
            options={{
              defaultValue: "No modpacks available",
            }}
          />
        </p>
      </div>
    </div>
  );
};

const FetchingModpacks = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_modpacks_text"
            options={{
              defaultValue: "Loading modpacks",
            }}
          />
        </p>
        <Spinner />
      </div>
    </div>
  );
};

const NoMoreModpacks = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans key="instance.fetching_no_more_modpacks" />
        </p>
      </div>
    </div>
  );
};

export { NoMoreModpacks, NoModpacksAvailable, FetchingModpacks };
