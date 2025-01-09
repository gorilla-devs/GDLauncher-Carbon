import { Trans } from "@gd/i18n"
import { Spinner } from "@gd/ui"
import skull from "/assets/images/icons/skull.png"
import { RSPCError } from "@rspc/client"

const NoModpacksAvailable = () => {
  return (
    <div class="bg-darkSlate-700 h-100 mx-5 flex flex-col items-center justify-center gap-4 rounded-xl">
      <div class="flex flex-col items-center justify-center text-center">
        <img src={skull} class="h-16 w-16" />

        <p class="text-lightSlate-700 max-w-100">
          <Trans
            key="instance.fetching_no_modpacks_available"
            options={{
              defaultValue: "No modpacks available"
            }}
          />
        </p>
      </div>
    </div>
  )
}

const FetchingModpacks = () => {
  return (
    <div class="flex h-56 flex-col items-center justify-center gap-4 rounded-xl p-5">
      <div class="flex flex-col items-center justify-center text-center">
        <p class="text-lightSlate-700 max-w-100">
          <Trans
            key="instance.fetching_modpacks_text"
            options={{
              defaultValue: "Loading modpacks"
            }}
          />
        </p>
        <Spinner />
      </div>
    </div>
  )
}

const NoMoreModpacks = () => {
  return (
    <div class="bg-darkSlate-700 flex h-56 flex-col items-center justify-center gap-4 rounded-xl p-5">
      <div class="flex flex-col items-center justify-center text-center">
        <p class="text-lightSlate-700 max-w-100">
          <Trans key="instance.fetching_no_more_modpacks" />
        </p>
      </div>
    </div>
  )
}

const ErrorFetchingModpacks = (props: { error: RSPCError | null }) => {
  const parsedError = () =>
    props.error?.message && JSON.parse(props.error?.message)
  return (
    <div class="min-h-90 flex h-full w-full items-center justify-center">
      <div class="flex flex-col items-center justify-center text-center">
        <p class="text-lightSlate-700 max-w-100">
          <Trans key="mods.fetching_mods_error" />
          {parsedError().cause[0].display}
        </p>
      </div>
    </div>
  )
}

export {
  NoMoreModpacks,
  NoModpacksAvailable,
  FetchingModpacks,
  ErrorFetchingModpacks
}
