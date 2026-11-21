import { JSX } from "solid-js"

// The list and the pane scroll on their own, so the tab takes the height
// left under the instance header, as the Logs tab does.
const WorldsRouteWrapper = (props: { children?: JSX.Element }) => {
  return <div style={{ height: "calc(100vh - 396px)" }}>{props.children}</div>
}

export default WorldsRouteWrapper
