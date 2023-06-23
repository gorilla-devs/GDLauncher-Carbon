import { JSX, children } from "solid-js";

type Props = {
  children: JSX.Element;
};

function PageTitle(props: Props) {
  const c = children(() => props.children);

  return <h3>{c()}</h3>;
}

export default PageTitle;
