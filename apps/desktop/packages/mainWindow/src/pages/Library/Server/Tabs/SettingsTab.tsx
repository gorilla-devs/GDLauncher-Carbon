import { Show } from "solid-js"
import useServerData from "../server.data"
import Settings from "../Settings"

const SettingsTab = () => {
  const routeData = useServerData()
  const details = () => routeData.serverDetails.data

  return (
    <Show when={details()}>
      {(d) => (
        <Settings
          serverDetails={d()}
          totalRam={
            routeData.totalRam.data
              ? parseInt(routeData.totalRam.data, 10)
              : undefined
          }
        />
      )}
    </Show>
  )
}

export default SettingsTab
