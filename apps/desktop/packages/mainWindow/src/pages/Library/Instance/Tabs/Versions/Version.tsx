import { Trans } from "@gd/i18n"
import { Show } from "solid-js"
import { format } from "date-fns"
import { CFFEFile } from "@gd/core_module/bindings"

interface Props {
  version: CFFEFile
  mainFileId: number
}

const getColor = (stable: string) => {
  switch (stable) {
    case "stable":
      return "text-green-500"
    case "beta":
      return "text-yellow-500"
    case "alpha":
      return "text-red-500"
    default:
      return "text-green-500"
  }
}

const Active = () => {
  return (
    <div class="flex items-center gap-2 text-green-500">
      <Trans
        key="instance.active_version"
        options={{
          defaultValue: "Active"
        }}
      />
      <div class="i-hugeicons:tick-02 text-2xl text-green-500" />
    </div>
  )
}

const Version = (props: Props) => {
  return (
    <div class="box-border flex h-14 w-full items-center py-2">
      <div class="flex w-full items-center justify-between gap-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex items-center gap-2">
            <div class="flex flex-col">
              <p class="mb-2 mt-0">{props.version.displayName}</p>
              <div class="flex gap-2">
                <div class="text-darkSlate-300 m-0 flex items-center gap-2 text-sm">
                  {props.version.gameVersions[1]}{" "}
                  {props.version.gameVersions[0]}
                  <div class="bg-darkSlate-300 h-2 w-px" />
                  <p class="text-darkSlate-300 text-md m-0">
                    {format(new Date(props.version.fileDate), "dd-MM-yyyy")}
                  </p>
                  <div class="bg-darkSlate-300 h-2 w-px" />
                  <span class={getColor(props.version.releaseType)}>
                    {props.version.releaseType}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <Show
          when={props.mainFileId === props.version.id}
          fallback={<Active />}
        >
          <div class="text-darkSlate-300 hover:text-lightSlate-700 group flex cursor-pointer items-center gap-2 transition ease-in-out">
            <Trans
              key="instance.switch_version"
              options={{
                defaultValue: "Switch Version"
              }}
            />
            <div class="i-hugeicons:download-02 text-darkSlate-300 group-hover:text-lightSlate-700 text-2xl" />
          </div>
        </Show>
      </div>
    </div>
  )
}

export default Version
