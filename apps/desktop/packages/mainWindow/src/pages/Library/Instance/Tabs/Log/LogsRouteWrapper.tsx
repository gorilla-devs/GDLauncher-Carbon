import { isFullScreen } from "."
import { JSX } from "solid-js"

const LogsRouteWrapper = (props: { children?: JSX.Element }) => {
  return (
    <div
      style={{
        height: isFullScreen() ? "calc(100vh - 155px)" : "calc(100vh - 396px)"
      }}
    >
      {props.children}
    </div>
  )
}

export default LogsRouteWrapper
