import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
};

function RowsContainer(props: Props) {
  const c = children(() => props.children);

  return (
    <div class="flex flex-col divide-y gap-4 divide-darkSlate-600">{c()}</div>
  );
}

export default RowsContainer;
