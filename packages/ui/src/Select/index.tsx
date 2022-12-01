import { children } from "solid-js";
import styles from "./Select.module.scss";

interface Props {
  children: HTMLElement | string;
};

function Select(props: Props) {
  const c = children(() => props.children);
  return <div class={styles.paolino}>{c()}</div>;
}

export { Select };
