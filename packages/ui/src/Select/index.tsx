import { children } from "solid-js";
interface Props {
  children: HTMLElement | string;
}

function Select(props: Props) {
  const c = children(() => props.children);
  return <div>{c()}</div>;
}

export { Select };
