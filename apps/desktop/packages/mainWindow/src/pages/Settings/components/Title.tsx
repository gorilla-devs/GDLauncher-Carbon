import { JSX, Show, children } from "solid-js";

type Props = {
  children: JSX.Element;
  description?: JSX.Element;
  class?: string;
};

function Title(props: Props) {
  const c = children(() => props.children);

  return (
    <div class={props.class || undefined}>
      <h4 class="text-lightSlate-100">{c()}</h4>
      <Show when={props.description}>
        <p class="text-lightSlate-800 max-w-110">{props.description}</p>
      </Show>
    </div>
  );
}

export default Title;
