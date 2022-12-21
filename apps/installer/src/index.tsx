/* @refresh reload */
import { render } from "solid-js/web";
import { appWindow } from "@tauri-apps/api/window";
import "./style.css";
import App from "./App";

addEventListener("DOMContentLoaded", (event) => {
  appWindow.show();
});

render(() => <App />, document.getElementById("root") as HTMLElement);
