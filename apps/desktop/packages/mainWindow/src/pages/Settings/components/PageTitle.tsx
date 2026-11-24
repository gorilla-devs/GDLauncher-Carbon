import { JSX, children } from "solid-js"
import { DecorationSlot, isDecorated } from "@/themes/DecorationSlot"

interface Props {
  children: JSX.Element
}

function PageTitle(props: Props) {
  const c = children(() => props.children)

  return (
    <h3
      data-gdl="page-title"
      class="my-4 text-2xl"
      classList={{ "relative isolate": isDecorated("page-title") }}
    >
      {c()}
      <DecorationSlot anchor="page-title" />
    </h3>
  )
}

export default PageTitle
