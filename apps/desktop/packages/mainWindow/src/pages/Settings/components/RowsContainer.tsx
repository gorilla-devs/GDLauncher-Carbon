import { JSX, children } from "solid-js"

interface Props {
  children: JSX.Element
}

function RowsContainer(props: Props) {
  const c = children(() => props.children)

  return <div class="divide-darkSlate-600 flex flex-col divide-y">{c()}</div>
}

export default RowsContainer
