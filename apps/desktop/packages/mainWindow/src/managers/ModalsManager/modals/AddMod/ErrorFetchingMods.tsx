import { Trans } from "@gd/i18n";
import { RSPCError } from "@rspc/client";

const ErrorFetchingMods = (props: { error: RSPCError | null }) => {
  const parsedError = () =>
    props.error?.message && JSON.parse(props.error?.message);
  return (
    <div class="w-full flex h-full justify-center items-center min-h-90">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans key="mods.fetching_mods_error" />
          {parsedError().cause[0].display}
        </p>
      </div>
    </div>
  );
};

export default ErrorFetchingMods;
