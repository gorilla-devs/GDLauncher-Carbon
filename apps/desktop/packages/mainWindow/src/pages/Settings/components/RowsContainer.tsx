import { JSX, children } from "solid-js"

interface Props {
  children: JSX.Element
}

function RowsContainer(props: Props) {
  const c = children(() => props.children)

  // The lines between the rows are in themes/controls.css, at no weight, so
  // a theme's plain `settings.row` rule replaces them.
  return (
    <div data-gdl="settings.rows" class="flex flex-col">
      {c()}
    </div>
  )
}

export default RowsContainer
