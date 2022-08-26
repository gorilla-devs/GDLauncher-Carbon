/* @refresh reload */
import { render } from "solid-js/web";
import App from "./app";
import "@gd/ui/style.css";
import "virtual:uno.css";

render(() => <App />, document.getElementById("root") as HTMLElement);
