import { JSX, Show, children } from "solid-js"
import { DecorationSlot, isDecorated } from "@/themes/DecorationSlot"

interface Props {
  children: JSX.Element
  description?: JSX.Element
  class?: string
}

function Title(props: Props) {
  const c = children(() => props.children)

  return (
    <div class={props.class || undefined}>
      {/* The same shape as a library group title: `section-title` holds a
          `section-title.text` element with the words. */}
      <h4
        data-gdl="section-title settings.row.title"
        class="text-lightSlate-100 my-2 text-lg font-medium"
        classList={{ "relative isolate": isDecorated("section-title") }}
      >
        <span data-gdl="section-title.text">{c()}</span>
        <DecorationSlot anchor="section-title" />
      </h4>
      <Show when={props.description}>
        <p
          data-gdl="settings.row.description"
          class="text-lightSlate-700 max-w-200 my-2 pr-4"
        >
          {props.description}
        </p>
      </Show>
    </div>
  )
}

export default Title
