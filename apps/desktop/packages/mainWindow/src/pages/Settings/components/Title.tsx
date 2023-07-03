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
      <h4 class="text-lg text-lightSlate-100 font-medium">{c()}</h4>
      <Show when={props.description}>
        <p class="text-lightSlate-800 max-w-200 pr-4">{props.description}</p>
      </Show>
    </div>
  );
}

export default Title;
