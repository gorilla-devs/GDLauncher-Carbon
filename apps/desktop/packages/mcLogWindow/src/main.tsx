/* @refresh reload */
import { render } from "solid-js/web";
import App from "./app";
import "tailwindcss/tailwind.css";
import "@gd/ui/style.css";

render(() => <App />, document.getElementById("root") as HTMLElement);
