import { JSX, children } from "solid-js"

interface Props {
  id?: string
  children: JSX.Element
  class?: string
  forceContentBelow?: boolean
  /** More hooks for this row, after `settings.row`. */
  hooks?: string
}

function Row(props: Props) {
  const c = children(() => props.children)

  return (
    <div
      data-gdl={props.hooks ? `settings.row ${props.hooks}` : "settings.row"}
      class={
        "py-6 flex justify-between items-center " +
          (props.forceContentBelow ? " flex-col items-stretch " : "") +
          props.class || ""
      }
      {...(props.id ? { id: props.id } : {})}
    >
      {c()}
    </div>
  )
}

export default Row
