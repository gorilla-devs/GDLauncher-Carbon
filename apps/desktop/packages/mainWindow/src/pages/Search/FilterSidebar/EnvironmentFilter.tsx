import { Collapsable, Radio } from "@gd/ui"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"
import { createUniqueId } from "solid-js"

export function EnvironmentFilter() {
  const searchResults = useSearchContext()
  // The environments are one radio group for the keyboard.
  const environmentGroup = createUniqueId()

  const handleSelect = (value: string | number | string[] | undefined) => {
    const env = value as "server" | "client"
    if (env === searchResults?.searchQuery().environment) {
      searchResults?.setSearchQuery((prev) => ({
        ...prev,
        environment: null
      }))
    } else {
      searchResults?.setSearchQuery((prev) => ({
        ...prev,
        environment: env
      }))
    }
  }

  return (
    <Collapsable
      title={
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:computer h-4 w-4" />
          <Trans key="search:_trn_environment" />
        </div>
      }
      defaultOpened
      noPadding
      count={searchResults?.searchQuery().environment ? 1 : 0}
      onClear={() => {
        searchResults?.setSearchQuery((prev) => ({
          ...prev,
          environment: null
        }))
      }}
    >
      <div class="flex flex-col px-2">
        <Radio
          name={environmentGroup}
          value="server"
          checked={searchResults?.searchQuery().environment === "server"}
          onChange={handleSelect}
          allowDeselect
        >
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:server-stack-01 h-4 w-4" />
            {capitalize("server")}
          </div>
        </Radio>
        <Radio
          name={environmentGroup}
          value="client"
          checked={searchResults?.searchQuery().environment === "client"}
          onChange={handleSelect}
          allowDeselect
        >
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:computer h-4 w-4" />
            {capitalize("client")}
          </div>
        </Radio>
      </div>
    </Collapsable>
  )
}
