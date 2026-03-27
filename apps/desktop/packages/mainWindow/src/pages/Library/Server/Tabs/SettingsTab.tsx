import { Show } from "solid-js"
import useServerData from "../server.data"
import Settings from "../Settings"

const SettingsTab = () => {
  const routeData = useServerData()
  const details = () => routeData.serverDetails.data

  return (
    <div class="h-full w-full overflow-y-auto">
      <Show when={details()}>
        {(d) => (
          <Settings
            serverDetails={d()}
            totalRam={routeData.totalRam.data ?? undefined}
          />
        )}
      </Show>
    </div>
  )
}

export default SettingsTab
