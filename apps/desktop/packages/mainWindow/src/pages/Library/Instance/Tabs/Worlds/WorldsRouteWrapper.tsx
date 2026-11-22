import { JSX, createSignal } from "solid-js"

/** The world map fills the tab: the instance page folds its header away, as
 *  for the Logs tab's full screen. */
export const [isMapFilled, setMapFilled] = createSignal(false)

// The list and the pane scroll on their own, so the tab takes the height
// left under the instance header, as the Logs tab does.
const WorldsRouteWrapper = (props: { children?: JSX.Element }) => {
  return (
    <div
      style={{
        height: isMapFilled() ? "calc(100vh - 155px)" : "calc(100vh - 396px)"
      }}
    >
      {props.children}
    </div>
  )
}

export default WorldsRouteWrapper
