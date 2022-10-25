import { children, JSX } from "solid-js";

type Props = {
  children: HTMLElement | string | JSX.Element;
  class?: string;
  style?: any;
  onClick?: () => void;
};

function Button(props: Props) {
  const c = children(() => props.children);
  return (
    <button
      class={props.class}
      style={{
        display: "flex",
        "justify-content": "center",
        "align-items": "center",
        "font-family": "ubuntu",
        color: "white",
        padding: "1rem 2rem",
        "max-width": "250px",
        "border-radius": "1rem",
        "background-color": "#2b6cb0",
        cursor: "pointer",
        "text-decoration": "none",
        "font-weight": "bold",
        height: "60px",
        border: 0,
        "box-sizing": "border-box",
        ...props.style,
      }}
    >
      {c()}
    </button>
  );
}

export default Button;
