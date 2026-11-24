import { JSX, children } from "solid-js"

interface Props {
  children: JSX.Element
  class?: string
}

function RightHandSide(props: Props) {
  const c = children(() => props.children)

  return (
    <div data-gdl="settings.row.control" class={props.class}>
      {c()}
    </div>
  )
}

export default RightHandSide
