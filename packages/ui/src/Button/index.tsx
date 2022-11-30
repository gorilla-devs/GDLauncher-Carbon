import { children } from "solid-js";
import styles from "./Button.module.scss";

type Props = {
  children: HTMLElement | string;
};
function Button(props: Props) {
  const c = children(() => props.children);
  return <div class={styles.paolo}>{c()}</div>;
}

export { Button };
