import { Outlet } from "@solidjs/router"
import { isFullScreen } from "."

const LogsRouteWrapper = () => {
  return (
    <div
      style={{
        height: isFullScreen() ? "calc(100vh - 155px)" : "calc(100vh - 396px)"
      }}
    >
      <Outlet />
    </div>
  )
}

export default LogsRouteWrapper
